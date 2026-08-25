mod client;
mod markdown;
mod table;

use anyhow::Result;
use colored::*;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::io::{self, Write};
use textwrap::wrap;

use crate::client::{ChatMessage, MistralClient};
use crate::markdown::MarkdownRenderer;
async fn chat_loop(client: MistralClient) -> Result<()> {
    let mut messages = Vec::new();

    // Get terminal width, default to 80 if unable to get it
    let width = match terminal_size::terminal_size() {
        Some((terminal_size::Width(w), _)) => w as usize - 2, // Subtract 2 for margin
        None => 80,
    };

    let renderer = MarkdownRenderer::new(width);

    // Define command box
    let command_box = "\
┌──────────────────────────────────────┐\n\
│          Available Commands          │\n\
├──────────────────────────────────────┤\n\
│    `exit`  - Quit the application    │\n\
├──────────────────────────────────────┤\n\
│    `clear` - Clear the screen        │\n\
├──────────────────────────────────────┤\n\
│    `new`   - Start a new chat        │\n\
└──────────────────────────────────────┘";

    // Function to show command box
    let show_command_box = || {
        println!("{}", command_box.green());
        println!();
    };

    // Show initial welcome message
    clearscreen::clear()?;
    if messages.is_empty() {
        let welcome_message = "I am Mistral Chat AI, a helpful and respectful assistant\npowered by Mistral. Here are some ways I can assist you:\n\n• Provide information and answer questions on a wide\nrange of topics\n• Generate ideas, suggestions, and recommendations\n\nI'm ready to help! How can I assist you today?";

        print!("{}", renderer.render(welcome_message).cyan());
        println!("\n");
        show_command_box();
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;
    } else {
        show_command_box();
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;
    }

    // Configure rustyline editor with history
    let mut rl = DefaultEditor::new()?;
    rl.set_max_history_size(100)?;

    // Load history from file if it exists
    let history_file = dirs::home_dir()
        .map(|mut path| {
            path.push(".mistral_history");
            path
        })
        .unwrap_or_else(|| ".mistral_history".into());

    if history_file.exists() {
        let _ = rl.load_history(&history_file);
    }

    loop {
        let prompt = format!("{}", "> ".blue().bold());
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.eq_ignore_ascii_case("exit") {
                    // Save history before exiting
                    let _ = rl.save_history(&history_file);
                    break;
                } else if input.eq_ignore_ascii_case("clear") {
                    clearscreen::clear()?;
                    show_command_box();
                    continue;
                } else if input.eq_ignore_ascii_case("new") {
                    messages.clear();
                    clearscreen::clear()?;
                    show_command_box();
                    println!("{}", "Starting a fresh conversation...".green());
                    print!("{}", "> ".blue().bold());
                    io::stdout().flush()?;
                    continue;
                }

                // Add valid input to history
                if !input.is_empty() {
                    rl.add_history_entry(input)?;
                }

                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: input.to_string(),
                });

                print!("{}", "Thinking...".yellow());
                io::stdout().flush()?;

                match client.send_message(messages.clone()).await {
                    Ok((response, language_hint)) => {
                        clearscreen::clear()?;
                        show_command_box();

                        print!("{}", "> ".blue().bold());
                        println!("{}", input);
                        println!();

                        print!(
                            "{}",
                            renderer
                                .render_with_hint(&response, language_hint.as_deref())
                                .cyan()
                        );
                        println!();
                        println!();

                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });

                        print!("{}", "> ".blue().bold());
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        print!("\r{}\r", " ".repeat(width)); // Clear "Thinking..." line
                        println!();
                        for line in wrap(&format!("Error: {}", e), &renderer.wrap_options) {
                            println!("{}", line.red());
                        }
                        println!();
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Use 'exit' to quit");
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let api_key = env::var("MISTRAL_API_KEY")
        .expect("MISTRAL_API_KEY must be set in environment variables or .env file");

    let model = env::var("MISTRAL_MODEL").unwrap_or_else(|_| "mistral-small".to_string());

    let client = MistralClient::new(api_key, model);
    chat_loop(client).await?;

    Ok(())
}
