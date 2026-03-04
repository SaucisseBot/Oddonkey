use serde::{Deserialize, Serialize};

use crate::domain::message::ChatMessage;
use crate::domain::options::GenerationOptions;

// ── Request types ───────────────────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub(crate) struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerationOptions>,
}

#[derive(Serialize, Debug)]
pub(crate) struct EmbedRequest {
    pub model: String,
    pub input: Vec<String>,
}

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub(crate) struct ChatResponse {
    pub message: ChatResponseMessage,
}

#[derive(Deserialize, Debug)]
pub(crate) struct StreamChunk {
    pub message: Option<ChatResponseMessage>,
    pub done: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ChatResponseMessage {
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ModelEntry {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ModelList {
    pub models: Vec<ModelEntry>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
}
