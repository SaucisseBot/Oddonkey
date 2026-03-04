use async_trait::async_trait;

use crate::domain::error::OddOnkeyError;
use crate::domain::message::ChatMessage;
use crate::domain::options::GenerationOptions;

/// The primary port: everything OddOnkey needs from an LLM backend.
///
/// Today the only adapter is Ollama, but implementing this trait for
/// another backend (llama.cpp, OpenAI-compatible, etc.) would plug
/// straight in without touching the core.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request and return the full response text.
    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&GenerationOptions>,
    ) -> Result<String, OddOnkeyError>;

    /// Send a chat completion request and return a byte stream for
    /// token-by-token reading.
    async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&GenerationOptions>,
    ) -> Result<reqwest::Response, OddOnkeyError>;

    /// Compute embeddings for one or more texts.
    async fn embed(
        &self,
        model: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, OddOnkeyError>;

    /// List locally available models.
    async fn list_models(&self) -> Result<Vec<String>, OddOnkeyError>;
}
