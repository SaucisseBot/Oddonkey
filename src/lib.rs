//! # OddOnkey
//!
//! A dead-simple Ollama wrapper crate.
//!
//! * Auto-detects or installs Ollama.
//! * Auto-pulls the requested model if it isn't already available.
//! * Provides a tiny API: `add_preprompt()`, `prompt()`, `prompt_stream()`, `embed()`.
//! * **Optional progress bar** during model pull (feature `progress`).
//! * **Optional per-prompt report** with timing & token stats (feature `report` or runtime toggle).
//!
//! ```rust,no_run
//! use oddonkey::OddOnkey;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut model = OddOnkey::new("mistral").await.unwrap();
//!     model.add_preprompt("You are a helpful assistant.");
//!     let answer = model.prompt("What is 2+2?").await.unwrap();
//!     println!("{answer}");
//! }
//! ```

// ── Modules (hexagonal architecture) ────────────────────────────────────────

pub mod adapters;
pub mod core;
pub mod domain;
pub mod ports;

// ── Public re-exports (flat, backward-compatible API) ───────────────────────

pub use crate::adapters::ollama::stream::TokenStream;
pub use crate::core::builder::OddOnkeyBuilder;
pub use crate::core::oddonkey::OddOnkey;
pub use crate::domain::error::OddOnkeyError;
pub use crate::domain::message::ChatMessage;
pub use crate::domain::options::GenerationOptions;
pub use crate::domain::report::PromptReport;
pub use crate::ports::llm_provider::LlmProvider;

#[cfg(feature = "docker")]
pub use crate::adapters::docker::manager::DockerManager;
