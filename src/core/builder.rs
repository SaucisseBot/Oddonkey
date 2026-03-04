use std::time::Duration;

use reqwest::Client;

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
}

impl OddOnkeyBuilder {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            base_url: "http://127.0.0.1:11434".to_string(),
            progress: false,
            report: false,
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

    /// Build the [`OddOnkey`] instance: installs Ollama, starts the
    /// server, and pulls the model as needed.
    pub async fn build(self) -> Result<OddOnkey, OddOnkeyError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        // 1. Ensure ollama binary exists
        installer::ensure_ollama_installed()?;

        // 2. Ensure server is reachable
        installer::ensure_server_running(&client, &self.base_url, self.progress).await?;

        // 3. Ensure model is available
        pull::ensure_model_available(&client, &self.base_url, &self.model_name, self.progress)
            .await?;

        let provider = OllamaClient::new(client, self.base_url);

        Ok(OddOnkey::from_provider(
            Box::new(provider),
            self.model_name,
            self.progress,
            self.report,
        ))
    }
}
