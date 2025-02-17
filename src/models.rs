use serde::{Deserialize, Serialize};
use rustyline::error::ReadlineError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API error: {0}")]
    Api(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Environment error: {0}")]
    Environment(#[from] std::env::VarError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Terminal error: {0}")]
    Terminal(#[from] clearscreen::Error),
    #[error("Readline error: {0}")]
    Readline(String),
}

impl From<ReadlineError> for Error {
    fn from(err: ReadlineError) -> Self {
        Error::Readline(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>; 