pub mod client;
pub mod models;
pub mod renderer;
pub mod ui;

pub use client::MistralClient;
pub use models::{ChatMessage, ChatRequest, ChatResponse, Choice};