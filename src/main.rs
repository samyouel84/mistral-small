use anyhow::Result;
use colored::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use textwrap::{wrap, Options};
use tokio;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

struct MistralClient {
    client: reqwest::Client,
    api_key: String,
}

impl MistralClient {
    fn new(api_key: String) -> Self {
        let client = reqwest::Client::new();
        Self { client, api_key }
    }

    async fn send_message(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let request = ChatRequest {
            model: "mistral-small".to_string(),
            messages,
        };

        let response = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .headers(headers)
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}

async fn chat_loop(client: MistralClient) -> Result<()> {
    let mut messages = Vec::new();
    
    // Clear the screen
    clearscreen::clear()?;
    
    // Get terminal width, default to 80 if unable to get it
    let width = match terminal_size::terminal_size() {
        Some((terminal_size::Width(w), _)) => w as usize - 2, // Subtract 2 for margin
        None => 80,
    };

    let wrap_options = Options::new(width)
        .initial_indent("  ")
        .subsequent_indent("  ");

    // Print wrapped welcome message
    let welcome_msg = "Welcome to Mistral Chat! Type your message ('exit' to quit, 'clear' to clear screen):";
    for line in wrap(welcome_msg, &wrap_options) {
        println!("{}", line.green());
    }
    println!();

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
                    Ok(response) => {
                        // Clear the screen and print from the top
                        clearscreen::clear()?;
                        
                        // Print the user's question
                        print!("{}", "> ".blue().bold());
                        println!("{}", input);
                        println!();
                        
                        // Print the response
                        let response_lines: Vec<_> = wrap(&response, &wrap_options).into_iter().collect();
                        for line in response_lines {
                            println!("{}", line.cyan());
                        }
                        println!();

                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });
                        
                        // Print prompt for next input
                        print!("{}", "> ".blue().bold());
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        print!("\r{}\r", " ".repeat(width)); // Clear "Thinking..." line
                        println!();
                        for line in wrap(&format!("Error: {}", e), &wrap_options) {
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

    let client = MistralClient::new(api_key);
    chat_loop(client).await?;

    Ok(())
}
