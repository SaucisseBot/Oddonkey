use std::time::Instant;

#[cfg(feature = "docker")]
use crate::adapters::docker::manager::DockerManager;
use crate::adapters::ollama::stream::TokenStream;
use crate::core::builder::OddOnkeyBuilder;
use crate::domain::error::OddOnkeyError;
use crate::domain::message::ChatMessage;
use crate::domain::options::GenerationOptions;
use crate::domain::report::{estimate_tokens, PromptReport};
use crate::ports::llm_provider::LlmProvider;

/// Handle to an LLM model that is guaranteed to be ready to use.
///
/// The core struct is backend-agnostic: it depends on the [`LlmProvider`]
/// port, not on any concrete adapter.
pub struct OddOnkey {
    model: String,
    provider: Box<dyn LlmProvider>,
    system_prompts: Vec<String>,
    history: Vec<ChatMessage>,
    options: Option<GenerationOptions>,
    #[allow(dead_code)]
    progress: bool,
    report_enabled: bool,
    last_report: Option<PromptReport>,
    #[cfg(feature = "docker")]
    docker_cleanup: bool,
}

impl OddOnkey {
    // ── Constructors ────────────────────────────────────────────────────

    /// Quick constructor – uses the Ollama adapter with default settings.
    ///
    /// ```rust,no_run
    /// # use oddonkey::OddOnkey;
    /// # async fn example() {
    /// let mut model = OddOnkey::new("mistral").await.unwrap();
    /// # }
    /// ```
    pub async fn new(model_name: &str) -> Result<Self, OddOnkeyError> {
        Self::builder(model_name).build().await
    }

    /// Same as [`new`](Self::new) but with a custom Ollama URL.
    pub async fn with_base_url(model_name: &str, base_url: &str) -> Result<Self, OddOnkeyError> {
        Self::builder(model_name).base_url(base_url).build().await
    }

    /// Create a [`OddOnkeyBuilder`] for fine-grained configuration.
    pub fn builder(model_name: &str) -> OddOnkeyBuilder {
        OddOnkeyBuilder::new(model_name)
    }

    /// Create from an already-initialised provider (used by the builder).
    pub(crate) fn from_provider(
        provider: Box<dyn LlmProvider>,
        model: String,
        progress: bool,
        report: bool,
        #[cfg(feature = "docker")] docker_cleanup: bool,
    ) -> Self {
        Self {
            model,
            provider,
            system_prompts: Vec::new(),
            history: Vec::new(),
            options: None,
            progress,
            report_enabled: report,
            last_report: None,
            #[cfg(feature = "docker")]
            docker_cleanup,
        }
    }

    // ── Options ─────────────────────────────────────────────────────────

    /// Set default generation options for all subsequent prompts.
    pub fn set_options(&mut self, opts: GenerationOptions) {
        self.options = Some(opts);
    }

    /// Clear generation options (revert to model defaults).
    pub fn clear_options(&mut self) {
        self.options = None;
    }

    // ── Progress & report ───────────────────────────────────────────────

    /// Enable or disable the progress spinner/bar.
    pub fn enable_progress(&mut self, on: bool) {
        self.progress = on;
    }

    /// Enable or disable the per-prompt report.
    pub fn enable_report(&mut self, on: bool) {
        self.report_enabled = on;
    }

    /// Retrieve the report from the most recent `prompt()` call.
    pub fn last_report(&self) -> Option<&PromptReport> {
        self.last_report.as_ref()
    }

    // ── Pre-prompts ─────────────────────────────────────────────────────

    /// Add a system-level pre-prompt.
    pub fn add_preprompt(&mut self, text: &str) {
        self.system_prompts.push(text.to_string());
    }

    /// Replace all existing pre-prompts with a single one.
    pub fn set_preprompt(&mut self, text: &str) {
        self.system_prompts.clear();
        self.system_prompts.push(text.to_string());
    }

    /// Clear conversation history (keep pre-prompts).
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    // ── Prompt (full response) ──────────────────────────────────────────

    /// Send a user prompt and return the assistant's text reply.
    pub async fn prompt(&mut self, user_message: &str) -> Result<String, OddOnkeyError> {
        self.prompt_with(user_message, self.options.clone()).await
    }

    /// Like [`prompt`](Self::prompt) but with per-call generation options.
    pub async fn prompt_with(
        &mut self,
        user_message: &str,
        options: Option<GenerationOptions>,
    ) -> Result<String, OddOnkeyError> {
        let messages = self.build_messages(user_message);

        let request_bytes = serde_json::to_string(&messages).unwrap_or_default().len();
        let start = Instant::now();

        let assistant_content = self
            .provider
            .chat(&self.model, &messages, options.as_ref())
            .await?;

        let duration = start.elapsed();
        let response_bytes = assistant_content.len();

        // Build report
        if self.report_enabled {
            let prompt_tokens_est = estimate_tokens(user_message);
            let completion_tokens_est = estimate_tokens(&assistant_content);
            let secs = duration.as_secs_f64().max(0.001);

            self.last_report = Some(PromptReport {
                duration,
                prompt_tokens_est,
                completion_tokens_est,
                tokens_per_sec: completion_tokens_est as f64 / secs,
                request_bytes,
                response_bytes,
                model: self.model.clone(),
            });
        }

        // Persist in history
        self.history.push(ChatMessage::user(user_message));
        self.history
            .push(ChatMessage::assistant(&assistant_content));

        Ok(assistant_content)
    }

    // ── Prompt (streaming) ──────────────────────────────────────────────

    /// Send a user prompt and get a [`TokenStream`] that yields tokens as
    /// they arrive.
    ///
    /// Call [`push_assistant_message`](Self::push_assistant_message) after
    /// collecting the stream to keep history.
    pub async fn prompt_stream(
        &mut self,
        user_message: &str,
    ) -> Result<TokenStream, OddOnkeyError> {
        self.prompt_stream_with(user_message, self.options.clone())
            .await
    }

    /// Like [`prompt_stream`](Self::prompt_stream) but with per-call options.
    pub async fn prompt_stream_with(
        &mut self,
        user_message: &str,
        options: Option<GenerationOptions>,
    ) -> Result<TokenStream, OddOnkeyError> {
        let messages = self.build_messages(user_message);

        let resp = self
            .provider
            .chat_stream(&self.model, &messages, options.as_ref())
            .await?;

        Ok(TokenStream::new(Box::pin(resp.bytes_stream())))
    }

    /// Manually add a user + assistant exchange to history.
    pub fn push_assistant_message(&mut self, user_message: &str, assistant_reply: &str) {
        self.history.push(ChatMessage::user(user_message));
        self.history.push(ChatMessage::assistant(assistant_reply));
    }

    // ── One-shot prompt ─────────────────────────────────────────────────

    /// Send a one-shot prompt **without** history or pre-prompts.
    pub async fn prompt_once(&self, user_message: &str) -> Result<String, OddOnkeyError> {
        let messages = vec![ChatMessage::user(user_message)];
        self.provider
            .chat(&self.model, &messages, self.options.as_ref())
            .await
    }

    // ── Embeddings ──────────────────────────────────────────────────────

    /// Compute an embedding vector for a single text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, OddOnkeyError> {
        let mut vecs = self.embed_batch(&[text]).await?;
        vecs.pop()
            .ok_or_else(|| OddOnkeyError::Parse("empty embedding response".into()))
    }

    /// Compute embeddings for multiple texts in a single request.
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, OddOnkeyError> {
        let owned: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
        self.provider.embed(&self.model, &owned).await
    }

    // ── Utilities ───────────────────────────────────────────────────────

    /// Return the model name this instance is using.
    pub fn model_name(&self) -> &str {
        &self.model
    }

    /// List all models locally available on the backend.
    pub async fn list_models(&self) -> Result<Vec<String>, OddOnkeyError> {
        self.provider.list_models().await
    }

    // ── Internals ───────────────────────────────────────────────────────

    fn build_messages(&self, user_message: &str) -> Vec<ChatMessage> {
        let mut messages: Vec<ChatMessage> = Vec::new();

        for sp in &self.system_prompts {
            messages.push(ChatMessage::system(sp));
        }

        messages.extend(self.history.clone());
        messages.push(ChatMessage::user(user_message));

        messages
    }
}

#[cfg(feature = "docker")]
impl Drop for OddOnkey {
    fn drop(&mut self) {
        if self.docker_cleanup {
            eprintln!("[oddonkey] cleaning up Docker container…");
            let mgr = DockerManager::new();
            if let Err(e) = mgr.destroy() {
                eprintln!("[oddonkey] docker cleanup failed: {e}");
            }
        }
    }
}
