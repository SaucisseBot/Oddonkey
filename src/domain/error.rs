/// Errors that OddOnkey can produce.
#[derive(Debug)]
pub enum OddOnkeyError {
    /// Ollama could not be installed automatically.
    InstallFailed(String),
    /// Ollama server did not become reachable in time.
    ServerStartFailed(String),
    /// Model pull failed.
    ModelPullFailed(String),
    /// HTTP / network error while talking to the LLM backend.
    Http(reqwest::Error),
    /// The response body could not be parsed.
    Parse(String),
}

impl std::fmt::Display for OddOnkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InstallFailed(s) => write!(f, "ollama install failed: {s}"),
            Self::ServerStartFailed(s) => write!(f, "ollama server start failed: {s}"),
            Self::ModelPullFailed(s) => write!(f, "model pull failed: {s}"),
            Self::Http(e) => write!(f, "http error: {e}"),
            Self::Parse(s) => write!(f, "parse error: {s}"),
        }
    }
}

impl std::error::Error for OddOnkeyError {}

impl From<reqwest::Error> for OddOnkeyError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}
