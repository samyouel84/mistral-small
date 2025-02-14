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
use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

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

struct MarkdownRenderer<'a> {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    wrap_options: Options<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(width: usize) -> Self {
        let wrap_options = Options::new(width)
            .initial_indent("  ")
            .subsequent_indent("  ");
            
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            wrap_options,
        }
    }

    fn render(&self, text: &str) -> String {
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let parser = Parser::new(text);
        let mut output = String::new();
        let mut in_code_block = false;
        let mut in_list = false;
        let mut current_paragraph = String::new();
        let mut current_language = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
                    in_code_block = true;
                    current_language = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        _ => "txt".to_string(),
                    };
                    output.push('\n');
                }
                Event::End(Tag::CodeBlock(_)) => {
                    in_code_block = false;
                    current_language.clear();
                    output.push('\n');
                }
                Event::Start(Tag::List(_)) => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
                    in_list = true;
                }
                Event::End(Tag::List(_)) => {
                    in_list = false;
                    output.push('\n');
                }
                Event::Start(Tag::Item) => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
                    current_paragraph.push_str("• ");
                }
                Event::End(Tag::Item) => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
                }
                Event::Start(Tag::Paragraph) => {
                    if !current_paragraph.is_empty() {
                        self.flush_paragraph(&mut output, &mut current_paragraph);
                    }
                }
                Event::End(Tag::Paragraph) => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
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
                        let syntax = if current_language.is_empty() {
                            self.syntax_set.find_syntax_plain_text()
                        } else {
                            self.syntax_set
                                .find_syntax_by_token(&current_language)
                                .or_else(|| self.syntax_set.find_syntax_by_extension(&current_language))
                                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
                        };

                        let mut highlighter = HighlightLines::new(syntax, theme);
                        
                        for line in LinesWithEndings::from(&text) {
                            match highlighter.highlight_line(line, &self.syntax_set) {
                                Ok(ranges) => {
                                    output.push_str("    "); // Indent
                                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                                    output.push_str(&escaped);
                                }
                                Err(_) => {
                                    output.push_str("    ");
                                    output.push_str(line);
                                }
                            }
                        }
                    } else {
                        current_paragraph.push_str(&text);
                    }
                }
                Event::SoftBreak => {
                    current_paragraph.push(' ');
                }
                Event::HardBreak => {
                    self.flush_paragraph(&mut output, &mut current_paragraph);
                    output.push('\n');
                }
                _ => {}
            }
        }

        self.flush_paragraph(&mut output, &mut current_paragraph);
        output.trim_end().to_string()
    }

    fn flush_paragraph(&self, output: &mut String, current: &mut String) {
        if !current.is_empty() {
            if current.starts_with('•') {
                // For list items, use special indentation
                let mut list_options = self.wrap_options.clone();
                list_options.initial_indent = "  ";  // 2 spaces for initial bullet
                list_options.subsequent_indent = "    "; // 4 spaces for wrapped lines
                for line in wrap(current, &list_options) {
                    output.push_str(&line);
                    output.push('\n');
                }
            } else {
                // For normal paragraphs
                for line in wrap(current, &self.wrap_options) {
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            current.clear();
        }
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

    let renderer = MarkdownRenderer::new(width);

    // Print wrapped welcome message with markdown list
    let welcome_msg = "Welcome to Mistral Chat!\n\nAvailable commands:\n* `exit` - quit the application\n* `clear` - clear the screen\n* `new` - start a fresh conversation\n\nType your message:";
    println!("{}", renderer.render(welcome_msg).green());
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
                        clearscreen::clear()?;
                        
                        print!("{}", "> ".blue().bold());
                        println!("{}", input);
                        println!();
                        
                        print!("{}", renderer.render(&response).cyan());
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

    let client = MistralClient::new(api_key);
    chat_loop(client).await?;

    Ok(())
}
