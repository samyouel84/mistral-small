mod client;
mod models;
mod renderer;
mod ui;

use anyhow::{Context, Result};
use client::MistralClient;
use std::env;
use ui::TerminalUI;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    let api_key = env::var("MISTRAL_API_KEY")
        .context("MISTRAL_API_KEY must be set in environment variables or .env file")?;

    let client = MistralClient::new(api_key);
    let mut terminal = TerminalUI::new(client)
        .context("Failed to initialize terminal UI")?;
    terminal.run().await
        .context("Error running terminal UI")?;

    Ok(())
}
