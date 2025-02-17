use crate::models::{ChatMessage, Result};
use crate::renderer::MarkdownRenderer;
use crate::client::MistralClient;
use super::commands::{Command, COMMAND_BOX};
use colored::*;
use rustyline::{config::Configurer, DefaultEditor, error::ReadlineError};
use std::io::{self, Write};
use std::path::PathBuf;
use terminal_size::{terminal_size, Width};

const WELCOME_MESSAGE: &str = "I am Mistral Chat AI, a helpful and respectful assistant\n\
powered by Mistral. Here are some ways I can assist you:\n\n\
• Provide information and answer questions on a wide\n\
range of topics\n\
• Generate ideas, suggestions, and recommendations\n\n\
I'm ready to help! How can I assist you today?";

pub struct TerminalUI {
    client: MistralClient,
    messages: Vec<ChatMessage>,
    renderer: MarkdownRenderer,
    editor: DefaultEditor,
    history_file: PathBuf,
    width: usize,
}

impl TerminalUI {
    pub fn new(client: MistralClient) -> Result<Self> {
        let width = match terminal_size() {
            Some((Width(w), _)) => w as usize - 2,
            None => 80,
        };

        let mut editor = DefaultEditor::new()?;
        editor.set_max_history_size(100)?;

        let history_file = dirs::home_dir()
            .map(|mut path| {
                path.push(".mistral_history");
                path
            })
            .unwrap_or_else(|| ".mistral_history".into());

        if history_file.exists() {
            let _ = editor.load_history(&history_file);
        }

        Ok(Self {
            client,
            messages: Vec::new(),
            renderer: MarkdownRenderer::new(width),
            editor,
            history_file,
            width,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.show_welcome_message()?;

        loop {
            let prompt = format!("{}", "> ".blue().bold());
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let command = line.parse::<Command>().unwrap_or_else(|_| Command::Message(line));
                    match command {
                        Command::Exit => {
                            let _ = self.editor.save_history(&self.history_file);
                            break;
                        }
                        Command::Clear => {
                            clearscreen::clear()?;
                            self.show_command_box();
                        }
                        Command::New => {
                            self.messages.clear();
                            clearscreen::clear()?;
                            self.show_command_box();
                            println!("{}", "Starting a fresh conversation...".green());
                            print!("{}", "> ".blue().bold());
                            io::stdout().flush()?;
                        }
                        Command::Message(input) => {
                            if !input.trim().is_empty() {
                                self.editor.add_history_entry(&input)?;
                                self.handle_message(&input).await?;
                            }
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'exit' to quit");
                    continue;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    fn show_welcome_message(&self) -> Result<()> {
        clearscreen::clear()?;
        print!("{}", self.renderer.render(WELCOME_MESSAGE).cyan());
        println!("\n");
        self.show_command_box();
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;
        Ok(())
    }

    fn show_command_box(&self) {
        println!("{}", COMMAND_BOX.green());
        println!();
    }

    async fn handle_message(&mut self, input: &str) -> Result<()> {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });

        print!("{}", "Thinking...".yellow());
        io::stdout().flush()?;

        match self.client.send_message(self.messages.clone()).await {
            Ok((response, language_hint)) => {
                clearscreen::clear()?;
                self.show_command_box();
                
                print!("{}", "> ".blue().bold());
                println!("{}", input);
                println!();
                
                print!("{}", self.renderer.render_with_hint(&response, language_hint.as_deref()).cyan());
                println!();
                println!();

                self.messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response,
                });
                
                print!("{}", "> ".blue().bold());
                io::stdout().flush()?;
            }
            Err(e) => {
                print!("\r{}\r", " ".repeat(self.width));
                println!();
                println!("{}", format!("Error: {}", e).red());
                println!();
            }
        }

        Ok(())
    }
}