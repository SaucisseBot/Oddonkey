use std::time::Duration;

/// Statistics collected after a prompt completes.
///
/// Enable with [`OddOnkey::enable_report()`]. After each `prompt()` /
/// `prompt_with()` call, retrieve it with [`OddOnkey::last_report()`].
#[derive(Clone, Debug)]
pub struct PromptReport {
    /// Wall-clock time for the full HTTP round-trip.
    pub duration: Duration,
    /// Number of tokens in the prompt (estimated).
    pub prompt_tokens_est: usize,
    /// Number of tokens in the response (estimated).
    pub completion_tokens_est: usize,
    /// Tokens per second for generation (estimated).
    pub tokens_per_sec: f64,
    /// Byte size of the serialised request body.
    pub request_bytes: usize,
    /// Byte size of the raw response body.
    pub response_bytes: usize,
    /// Model name used.
    pub model: String,
}

impl std::fmt::Display for PromptReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ms = self.duration.as_millis();
        write!(
            f,
            "── report ──────────────────────────────────\n\
             model           : {}\n\
             duration        : {ms} ms\n\
             prompt tokens   : ~{} (est.)\n\
             completion tkns : ~{} (est.)\n\
             tokens/sec      : {:.1}\n\
             request size    : {} bytes\n\
             response size   : {} bytes\n\
             ────────────────────────────────────────────",
            self.model,
            self.prompt_tokens_est,
            self.completion_tokens_est,
            self.tokens_per_sec,
            self.request_bytes,
            self.response_bytes,
        )
    }
}

/// Rough token estimate (~1.3 tokens per whitespace-delimited word).
pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    (words as f64 * 1.3).ceil() as usize
}
