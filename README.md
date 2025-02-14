# Mistral Chat CLI

A command-line interface for interacting with Mistral AI's language models, featuring rich markdown rendering and advanced syntax highlighting.

## Features

* 🎨 **Rich Syntax Highlighting**
  * Support for 50+ programming languages and file formats
  * Context-aware language detection from user queries
  * Automatic fallback for unknown languages
  * Categories include:
    - Systems Programming (Rust, C++, C#, etc.)
    - Web Development (JavaScript, TypeScript, HTML, CSS)
    - Scripting Languages (Python, Ruby, Perl)
    - JVM Languages (Java, Kotlin, Scala)
    - Mobile Development (Swift, Flutter/Dart)
    - Data & ML (R, Julia, MATLAB)
    - And many more!

* 📝 **Markdown Rendering**
  * Beautiful terminal formatting
  * Support for lists, code blocks, and emphasis
  * Proper indentation and text wrapping

* 💻 **Terminal Features**
  * Command history with persistent storage
  * Clear screen and conversation management
  * Proper terminal width detection
  * Color-coded output

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Create a `.env` file with your Mistral API key:
   ```
   MISTRAL_API_KEY=your_api_key_here
   ```
4. Build and run:
   ```bash
   cargo build --release
   cargo run
   ```

## Usage

Available commands:
* `exit` - quit the application
* `clear` - clear the screen
* `new` - start a fresh conversation

Simply type your questions or prompts, and the AI will respond with properly formatted and syntax-highlighted responses.

For code-related queries, you can mention the programming language in your question to get proper syntax highlighting:
```
> Show me a Python function for bubble sort
> Write a Rust example of reading a file
> Give me a JavaScript fetch example
```

## Configuration

The application stores its configuration in:
* Command history: `~/.mistral_history`
* API Key: `.env` file in the project directory

## Requirements

* Rust 1.70 or higher
* Terminal with ANSI color support
* Internet connection for API access

## License

This project is licensed under the MIT Licence - see the LICENCE file for details.

## Author
Samuel Morrison

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 
