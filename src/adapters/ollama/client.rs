use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::adapters::ollama::types::{
    ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, ModelList,
};
use crate::domain::error::OddOnkeyError;
use crate::domain::message::ChatMessage;
use crate::domain::options::GenerationOptions;
use crate::ports::llm_provider::LlmProvider;

/// Adapter that talks to a local Ollama HTTP server.
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(client: Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http_client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl LlmProvider for OllamaClient {
    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&GenerationOptions>,
    ) -> Result<String, OddOnkeyError> {
        let body = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: false,
            options: options.cloned(),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(OddOnkeyError::Parse(format!(
                "Ollama returned {status}: {text}"
            )));
        }

        let parsed: ChatResponse = serde_json::from_str(&text)
            .map_err(|e| OddOnkeyError::Parse(format!("{e} – raw: {text}")))?;

        Ok(parsed.message.content)
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: Option<&GenerationOptions>,
    ) -> Result<reqwest::Response, OddOnkeyError> {
        let body = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: true,
            options: options.cloned(),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .timeout(Duration::from_secs(600))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(OddOnkeyError::Parse(format!(
                "Ollama returned {status}: {text}"
            )));
        }

        Ok(resp)
    }

    async fn embed(
        &self,
        model: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, OddOnkeyError> {
        let body = EmbedRequest {
            model: model.to_string(),
            input: texts.to_vec(),
        };

        let resp = self
            .client
            .post(format!("{}/api/embed", self.base_url))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(OddOnkeyError::Parse(format!(
                "Ollama returned {status}: {text}"
            )));
        }

        let parsed: EmbedResponse = serde_json::from_str(&text)
            .map_err(|e| OddOnkeyError::Parse(format!("{e} – raw: {text}")))?;

        Ok(parsed.embeddings)
    }

    async fn list_models(&self) -> Result<Vec<String>, OddOnkeyError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let list: ModelList = resp
            .json()
            .await
            .map_err(|e| OddOnkeyError::Parse(format!("failed to parse model list: {e}")))?;

        Ok(list.models.into_iter().map(|m| m.name).collect())
    }
}
