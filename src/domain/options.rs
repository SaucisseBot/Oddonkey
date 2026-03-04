use serde::Serialize;

/// Controls how the model generates text.
///
/// All fields are optional – only set the ones you care about.
///
/// ```rust
/// use oddonkey::GenerationOptions;
///
/// let opts = GenerationOptions::default()
///     .temperature(0.3)
///     .num_ctx(8192);
/// ```
#[derive(Clone, Debug, Serialize, Default)]
pub struct GenerationOptions {
    /// Sampling temperature (0.0 = deterministic, 1.0+ = creative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Context window size in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
    /// Top-p (nucleus) sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Repetition penalty (1.0 = no penalty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
    /// Number of tokens to look back for repeat penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_last_n: Option<u32>,
    /// Random seed (for reproducibility).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Max tokens to generate (0 = unlimited).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
}

impl GenerationOptions {
    pub fn temperature(mut self, v: f32) -> Self {
        self.temperature = Some(v);
        self
    }
    pub fn num_ctx(mut self, v: u32) -> Self {
        self.num_ctx = Some(v);
        self
    }
    pub fn top_p(mut self, v: f32) -> Self {
        self.top_p = Some(v);
        self
    }
    pub fn top_k(mut self, v: u32) -> Self {
        self.top_k = Some(v);
        self
    }
    pub fn repeat_penalty(mut self, v: f32) -> Self {
        self.repeat_penalty = Some(v);
        self
    }
    pub fn repeat_last_n(mut self, v: u32) -> Self {
        self.repeat_last_n = Some(v);
        self
    }
    pub fn seed(mut self, v: u64) -> Self {
        self.seed = Some(v);
        self
    }
    pub fn num_predict(mut self, v: i32) -> Self {
        self.num_predict = Some(v);
        self
    }
}
