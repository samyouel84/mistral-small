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
use pulldown_cmark::{Parser, Event, Tag};

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

fn render_markdown(text: &str, wrap_options: &Options) -> String {
    let parser = Parser::new(text);
    let mut output = String::new();
    let mut in_code_block = false;
    let mut in_list = false;
    let mut current_paragraph = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                in_code_block = true;
                output.push('\n');
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                output.push('\n');
            }
            Event::Start(Tag::List(_)) => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                in_list = true;
            }
            Event::End(Tag::List(_)) => {
                in_list = false;
                output.push('\n');
            }
            Event::Start(Tag::Item) => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                current_paragraph.push_str("• ");
            }
            Event::End(Tag::Item) => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
            }
            Event::Start(Tag::Paragraph) => {
                if !current_paragraph.is_empty() {
                    flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                }
            }
            Event::End(Tag::Paragraph) => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                if !in_list {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Emphasis) => {
                current_paragraph.push_str("\x1B[3m"); // Italic
            }
            Event::End(Tag::Emphasis) => {
                current_paragraph.push_str("\x1B[23m"); // Reset italic
            }
            Event::Start(Tag::Strong) => {
                current_paragraph.push_str("\x1B[1m"); // Bold
            }
            Event::End(Tag::Strong) => {
                current_paragraph.push_str("\x1B[22m"); // Reset bold
            }
            Event::Code(text) => {
                current_paragraph.push('`');
                current_paragraph.push_str(&text);
                current_paragraph.push('`');
            }
            Event::Text(text) => {
                if in_code_block {
                    // Indent code blocks
                    for line in text.lines() {
                        output.push_str("    ");
                        output.push_str(line);
                        output.push('\n');
                    }
                } else {
                    current_paragraph.push_str(&text);
                }
            }
            Event::SoftBreak => {
                current_paragraph.push(' ');
            }
            Event::HardBreak => {
                flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
                output.push('\n');
            }
            _ => {}
        }
    }

    // Handle any remaining text
    flush_paragraph(&mut output, &mut current_paragraph, wrap_options);
    output.trim_end().to_string()
}

fn flush_paragraph(output: &mut String, current: &mut String, wrap_options: &Options) {
    if !current.is_empty() {
        if current.starts_with('•') {
            // For list items, use special indentation
            let mut list_options = wrap_options.clone();
            list_options.initial_indent = "  ";  // 2 spaces for initial bullet
            list_options.subsequent_indent = "    "; // 4 spaces for wrapped lines
            for line in wrap(current, &list_options) {
                output.push_str(&line);
                output.push('\n');
            }
        } else {
            // For normal paragraphs
            for line in wrap(current, wrap_options) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        current.clear();
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
    let welcome_msg = "Welcome to Mistral Chat!\n\nAvailable commands:\n  • exit  - quit the application\n  • clear - clear the screen\n  • new   - start a fresh conversation\n\nType your message:";
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
                } else if input.eq_ignore_ascii_case("new") {
                    messages.clear();
                    clearscreen::clear()?;
                    println!("{}", "Starting a fresh conversation...".green());
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
                        
                        // Print the response with markdown rendering
                        let rendered_response = render_markdown(&response, &wrap_options);
                        print!("{}", rendered_response.cyan());
                        println!();
                        println!();  // Add extra line break

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
