use std::time::Duration;

use reqwest::Client;

#[cfg(feature = "docker")]
use crate::adapters::docker::manager::DockerManager;
use crate::adapters::ollama::client::OllamaClient;
use crate::adapters::ollama::installer;
use crate::adapters::ollama::pull;
use crate::core::oddonkey::OddOnkey;
use crate::domain::error::OddOnkeyError;

/// Configures and creates an [`OddOnkey`] instance.
///
/// ```rust,no_run
/// # use oddonkey::OddOnkey;
/// # async fn example() {
/// let mut model = OddOnkey::builder("mistral")
///     .progress(true)
///     .report(true)
///     .build()
///     .await
///     .unwrap();
/// # }
/// ```
pub struct OddOnkeyBuilder {
    model_name: String,
    base_url: String,
    progress: bool,
    report: bool,
    #[cfg(feature = "docker")]
    docker: bool,
    #[cfg(feature = "docker")]
    docker_gpu: bool,
    #[cfg(feature = "docker")]
    docker_port: u16,
    #[cfg(feature = "docker")]
    docker_cleanup: bool,
}

impl OddOnkeyBuilder {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            base_url: "http://127.0.0.1:11434".to_string(),
            progress: false,
            report: false,
            #[cfg(feature = "docker")]
            docker: false,
            #[cfg(feature = "docker")]
            docker_gpu: false,
            #[cfg(feature = "docker")]
            docker_port: 11434,
            #[cfg(feature = "docker")]
            docker_cleanup: false,
        }
    }

    /// Override the Ollama server URL (default `http://127.0.0.1:11434`).
    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Enable a progress spinner during model pull / server start.
    ///
    /// Requires the `progress` Cargo feature – silently ignored otherwise.
    pub fn progress(mut self, on: bool) -> Self {
        self.progress = on;
        self
    }

    /// Enable the per-prompt [`PromptReport`](crate::domain::report::PromptReport).
    pub fn report(mut self, on: bool) -> Self {
        self.report = on;
        self
    }

    /// Run Ollama inside a Docker container instead of installing it
    /// locally. Requires the `docker` Cargo feature and Docker on the
    /// host.
    ///
    /// The container is named `oddonkey-ollama`, persists pulled models
    /// across restarts, and is automatically created/started as needed.
    #[cfg(feature = "docker")]
    pub fn docker(mut self, on: bool) -> Self {
        self.docker = on;
        self
    }

    /// Enable GPU passthrough (`--gpus=all`) for the Docker container.
    /// Requires the NVIDIA Container Toolkit.
    #[cfg(feature = "docker")]
    pub fn docker_gpu(mut self, on: bool) -> Self {
        self.docker_gpu = on;
        self
    }

    /// Override the host port mapped to the Docker container
    /// (default `11434`).
    #[cfg(feature = "docker")]
    pub fn docker_port(mut self, port: u16) -> Self {
        self.docker_port = port;
        self
    }

    /// When enabled, automatically stop and remove the Docker container
    /// (and its volume) when the `OddOnkey` instance is dropped.
    ///
    /// This leaves zero traces on the user's machine. Disabled by default
    /// so that pulled models persist across runs.
    #[cfg(feature = "docker")]
    pub fn docker_cleanup(mut self, on: bool) -> Self {
        self.docker_cleanup = on;
        self
    }

    /// Build the [`OddOnkey`] instance: installs Ollama, starts the
    /// server, and pulls the model as needed.
    ///
    /// When the `docker` feature is enabled and `.docker(true)` was
    /// called, Ollama runs inside a Docker container instead of being
    /// installed locally.
    pub async fn build(self) -> Result<OddOnkey, OddOnkeyError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        #[cfg(feature = "docker")]
        let base_url = if self.docker {
            let mut mgr = DockerManager::new()
                .host_port(self.docker_port)
                .gpu(self.docker_gpu);

            mgr.ensure_running(&client, self.progress).await?;

            // Pull the model inside the container
            let docker_base = mgr.base_url();
            pull::ensure_model_available(&client, &docker_base, &self.model_name, self.progress)
                .await?;

            docker_base
        } else {
            installer::ensure_ollama_installed()?;
            installer::ensure_server_running(&client, &self.base_url, self.progress).await?;
            pull::ensure_model_available(&client, &self.base_url, &self.model_name, self.progress)
                .await?;
            self.base_url.clone()
        };

        #[cfg(not(feature = "docker"))]
        let base_url = {
            installer::ensure_ollama_installed()?;
            installer::ensure_server_running(&client, &self.base_url, self.progress).await?;
            pull::ensure_model_available(&client, &self.base_url, &self.model_name, self.progress)
                .await?;
            self.base_url.clone()
        };

        let provider = OllamaClient::new(client, base_url);

        Ok(OddOnkey::from_provider(
            Box::new(provider),
            self.model_name,
            self.progress,
            self.report,
            #[cfg(feature = "docker")]
            self.docker_cleanup,
        ))
    }
}
