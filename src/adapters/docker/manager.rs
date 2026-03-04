use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

use crate::domain::error::OddOnkeyError;

const CONTAINER_NAME: &str = "oddonkey-ollama";
const DOCKER_IMAGE: &str = "ollama/ollama";
const DEFAULT_HOST_PORT: u16 = 11435;

/// Manages the lifecycle of an Ollama Docker container.
pub struct DockerManager {
    container_name: String,
    image: String,
    host_port: u16,
    /// Whether to mount a volume for persistent model storage.
    persist_models: bool,
    /// Whether to request GPU passthrough (--gpus=all).
    gpu: bool,
}

impl DockerManager {
    pub fn new() -> Self {
        Self {
            container_name: CONTAINER_NAME.to_string(),
            image: DOCKER_IMAGE.to_string(),
            host_port: DEFAULT_HOST_PORT,
            persist_models: true,
            gpu: false,
        }
    }

    /// Use a custom container name (default `oddonkey-ollama`).
    pub fn container_name(mut self, name: &str) -> Self {
        self.container_name = name.to_string();
        self
    }

    /// Use a custom Docker image (default `ollama/ollama`).
    pub fn image(mut self, image: &str) -> Self {
        self.image = image.to_string();
        self
    }

    /// Map to a different host port (default `11434`).
    pub fn host_port(mut self, port: u16) -> Self {
        self.host_port = port;
        self
    }

    /// Whether to persist pulled models across container restarts
    /// via a Docker volume (default `true`).
    pub fn persist_models(mut self, on: bool) -> Self {
        self.persist_models = on;
        self
    }

    /// Enable GPU passthrough (`--gpus=all`). Requires the NVIDIA
    /// Container Toolkit on the host (default `false`).
    pub fn gpu(mut self, on: bool) -> Self {
        self.gpu = on;
        self
    }

    /// The base URL the containerised Ollama will be reachable at.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.host_port)
    }

    // ── Lifecycle ───────────────────────────────────────────────────────

    /// Ensure Docker is available on the host.
    pub fn ensure_docker_installed() -> Result<(), OddOnkeyError> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .map_err(|e| {
                OddOnkeyError::InstallFailed(format!(
                    "docker not found – install Docker first: {e}"
                ))
            })?;

        if !output.status.success() {
            return Err(OddOnkeyError::InstallFailed(
                "docker --version failed".into(),
            ));
        }

        Ok(())
    }

    /// Return `true` if the container already exists (running or stopped).
    fn container_exists(&self) -> bool {
        Command::new("docker")
            .args(["inspect", "--format", "{{.Id}}", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Return `true` if the container is currently running.
    fn container_running(&self) -> bool {
        let output = Command::new("docker")
            .args([
                "inspect",
                "--format",
                "{{.State.Running}}",
                &self.container_name,
            ])
            .output();

        match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "true",
            Err(_) => false,
        }
    }

    /// Pull the Docker image if not already present locally.
    fn pull_image(&self, show_progress: bool) -> Result<(), OddOnkeyError> {
        // Check if image already exists locally
        let exists = Command::new("docker")
            .args(["image", "inspect", &self.image])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if exists {
            return Ok(());
        }

        eprintln!("[oddonkey/docker] pulling image '{}'…", self.image);

        #[cfg(feature = "progress")]
        let spinner = if show_progress {
            let sp = indicatif::ProgressBar::new_spinner();
            sp.set_style(
                indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            sp.set_message(format!("pulling {}…", self.image));
            sp.enable_steady_tick(Duration::from_millis(100));
            Some(sp)
        } else {
            None
        };

        #[cfg(not(feature = "progress"))]
        let _ = show_progress;

        let status = Command::new("docker")
            .args(["pull", &self.image])
            .status()
            .map_err(|e| OddOnkeyError::InstallFailed(format!("docker pull failed: {e}")))?;

        #[cfg(feature = "progress")]
        if let Some(sp) = &spinner {
            if status.success() {
                sp.finish_with_message(format!("{} pulled ✓", self.image));
            } else {
                sp.finish_with_message(format!("{} pull failed ✗", self.image));
            }
        }

        if !status.success() {
            return Err(OddOnkeyError::InstallFailed(format!(
                "docker pull {} failed",
                self.image
            )));
        }

        Ok(())
    }

    /// Start (or create + start) the Ollama container, then wait until
    /// the HTTP API is responsive.
    pub async fn ensure_running(
        &mut self,
        client: &reqwest::Client,
        show_progress: bool,
    ) -> Result<(), OddOnkeyError> {
        Self::ensure_docker_installed()?;
        self.pull_image(show_progress)?;

        if self.container_running() {
            return Ok(());
        }

        if self.container_exists() {
            // Container exists but stopped — try to start it
            eprintln!(
                "[oddonkey/docker] starting existing container '{}'…",
                self.container_name
            );
            let status = Command::new("docker")
                .args(["start", &self.container_name])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| {
                    OddOnkeyError::ServerStartFailed(format!("docker start failed: {e}"))
                })?;

            if !status.success() {
                // Container is in a broken state — remove it and recreate
                eprintln!("[oddonkey/docker] container broken, removing and recreating…");
                let _ = Command::new("docker")
                    .args(["rm", "-f", &self.container_name])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                self.create_container()?;
            }
        } else {
            self.create_container()?;
        }

        // Wait for the API to become responsive
        self.wait_for_api(client, show_progress).await
    }

    /// Create and start a new container.
    fn create_container(&mut self) -> Result<(), OddOnkeyError> {
        // Find an available port (the configured one or the next free one)
        let port = Self::find_available_port(self.host_port);
        self.host_port = port;

        eprintln!(
            "[oddonkey/docker] creating container '{}' from '{}' on port {}…",
            self.container_name, self.image, port
        );

        let mut args: Vec<String> = vec![
            "run".into(),
            "-d".into(),
            "--name".into(),
            self.container_name.clone(),
            "-p".into(),
            format!("{}:11434", port),
        ];

        if self.persist_models {
            args.push("-v".into());
            args.push(format!("{}-data:/root/.ollama", self.container_name));
        }

        if self.gpu {
            args.push("--gpus=all".into());
        }

        args.push(self.image.clone());

        let output = Command::new("docker")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| OddOnkeyError::ServerStartFailed(format!("docker run failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OddOnkeyError::ServerStartFailed(format!(
                "docker run failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Poll the API until it answers or we time out (30 s).
    async fn wait_for_api(
        &self,
        client: &reqwest::Client,
        show_progress: bool,
    ) -> Result<(), OddOnkeyError> {
        let url = format!("{}/api/tags", self.base_url());

        #[cfg(feature = "progress")]
        let spinner = if show_progress {
            let sp = indicatif::ProgressBar::new_spinner();
            sp.set_style(
                indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            sp.set_message("waiting for ollama container…");
            sp.enable_steady_tick(Duration::from_millis(100));
            Some(sp)
        } else {
            None
        };

        #[cfg(not(feature = "progress"))]
        let _ = show_progress;

        for i in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if client
                .get(&url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .is_ok()
            {
                #[cfg(feature = "progress")]
                if let Some(sp) = &spinner {
                    sp.finish_with_message(format!("container ready (took ~{}ms)", (i + 1) * 500));
                }
                eprintln!(
                    "[oddonkey/docker] container ready (took ~{}ms)",
                    (i + 1) * 500
                );
                return Ok(());
            }
        }

        #[cfg(feature = "progress")]
        if let Some(sp) = &spinner {
            sp.finish_with_message("container did not respond ✗");
        }

        Err(OddOnkeyError::ServerStartFailed(
            "docker container did not respond within 30 s".into(),
        ))
    }

    /// Stop the container (does not remove it, so models persist).
    pub fn stop(&self) -> Result<(), OddOnkeyError> {
        let status = Command::new("docker")
            .args(["stop", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| OddOnkeyError::ServerStartFailed(format!("docker stop failed: {e}")))?;

        if !status.success() {
            return Err(OddOnkeyError::ServerStartFailed(
                "docker stop exited with non-zero status".into(),
            ));
        }

        eprintln!(
            "[oddonkey/docker] container '{}' stopped.",
            self.container_name
        );
        Ok(())
    }

    /// Stop **and remove** the container + its volume.
    pub fn destroy(&self) -> Result<(), OddOnkeyError> {
        // Stop (ignore errors — might already be stopped)
        let _ = Command::new("docker")
            .args(["stop", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        let status = Command::new("docker")
            .args(["rm", "-v", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| OddOnkeyError::ServerStartFailed(format!("docker rm failed: {e}")))?;

        if !status.success() {
            return Err(OddOnkeyError::ServerStartFailed(
                "docker rm exited with non-zero status".into(),
            ));
        }

        eprintln!(
            "[oddonkey/docker] container '{}' removed.",
            self.container_name
        );
        Ok(())
    }
}

impl Default for DockerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerManager {
    /// Try to bind the given port; if taken, try the next 100 ports.
    fn find_available_port(preferred: u16) -> u16 {
        for p in preferred..preferred.saturating_add(100) {
            if TcpListener::bind(("127.0.0.1", p)).is_ok() {
                return p;
            }
        }
        // Fall back to the preferred and let Docker report the error
        preferred
    }
}
