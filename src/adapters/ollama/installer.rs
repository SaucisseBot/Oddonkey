use std::process::Command;

use crate::domain::error::OddOnkeyError;

/// Make sure the `ollama` binary is on PATH; install it if missing.
pub fn ensure_ollama_installed() -> Result<(), OddOnkeyError> {
    if which::which("ollama").is_ok() {
        return Ok(());
    }

    eprintln!("[oddonkey] ollama not found – installing…");

    let status = Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://ollama.com/install.sh | sh")
        .status()
        .map_err(|e| OddOnkeyError::InstallFailed(e.to_string()))?;

    if !status.success() {
        return Err(OddOnkeyError::InstallFailed(
            "install script exited with non-zero status".into(),
        ));
    }

    if which::which("ollama").is_err() {
        return Err(OddOnkeyError::InstallFailed(
            "ollama binary not found on PATH after install".into(),
        ));
    }

    Ok(())
}

/// Make sure the Ollama HTTP server is reachable; start it if it isn't.
pub async fn ensure_server_running(
    client: &reqwest::Client,
    base_url: &str,
    show_progress: bool,
) -> Result<(), OddOnkeyError> {
    use std::time::Duration;

    let url = format!("{base_url}/api/tags");

    // Quick check – already running?
    if client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .is_ok()
    {
        return Ok(());
    }

    eprintln!("[oddonkey] starting ollama server…");

    Command::new("ollama")
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| OddOnkeyError::ServerStartFailed(e.to_string()))?;

    #[cfg(feature = "progress")]
    let spinner = if show_progress {
        let sp = indicatif::ProgressBar::new_spinner();
        sp.set_style(
            indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        sp.set_message("waiting for ollama server…");
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
                sp.finish_with_message(format!("server ready (took ~{}ms)", (i + 1) * 500));
            }
            eprintln!("[oddonkey] server ready (took ~{}ms)", (i + 1) * 500);
            return Ok(());
        }
    }

    #[cfg(feature = "progress")]
    if let Some(sp) = &spinner {
        sp.finish_with_message("server did not respond ✗");
    }

    Err(OddOnkeyError::ServerStartFailed(
        "server did not respond within 30 s".into(),
    ))
}
