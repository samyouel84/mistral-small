# Mistral Chat CLI

A command-line interface for interacting with Mistral AI's language models, featuring rich markdown rendering, advanced syntax highlighting, and an intuitive chat interface.

## Features

* 🤖 **Mistral AI Integration**
  * Powered by Mistral's state-of-the-art language models
  * Natural conversational interface
  * Context-aware responses
  * Fast and efficient processing

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
  * Support for:
    - Tables with alignment (left, right, center)
    - Lists and nested lists
    - Code blocks with syntax highlighting
    - Text emphasis (bold, italic)
  * Proper indentation and text wrapping
  * Unicode box-drawing characters for tables

* 💻 **Terminal Features**
  * Command history with persistent storage
  * Clear screen and conversation management
  * Proper terminal width detection
  * Color-coded output
  * Intuitive command interface

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

The application provides a clean and intuitive interface with the following commands:
* `exit` - Quit the application
* `clear` - Clear the screen
* `new` - Start a fresh conversation

Simply type your questions or prompts, and Mistral AI will respond with properly formatted and syntax-highlighted responses.

### Example Interactions

The chat supports a wide range of queries and provides well-formatted responses:

1. **General Questions**
   ```
   > What are the benefits of cloud computing?
   > Can you explain how photosynthesis works?
   ```

2. **Programming Help**
   ```
   > Show me a Python function for bubble sort
   > Write a Rust example of reading a file
   > Explain the difference between let and const in JavaScript
   ```

3. **Creative Tasks**
   ```
   > Help me brainstorm ideas for a blog post
   > Suggest some names for a tech startup
   ```

### Markdown Features

The chat supports rich markdown formatting, including tables:

```markdown
| Feature | Description | Support |
|:--------|:----------:|--------:|
| Tables | Aligned columns | ✓ |
| Lists | Nested bullets | ✓ |
| Code | Syntax highlighting | ✓ |
```

## Configuration

The application stores its configuration in:
* Command history: `~/.mistral_history`
* API Key: `.env` file in the project directory

## Requirements

* Rust 1.70 or higher
* Terminal with ANSI color support
* Internet connection for API access
* Mistral API key

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Samuel Morrison

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change. 
