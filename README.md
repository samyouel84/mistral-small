# Mistral Chat CLI Rust

A lightning-fast command-line interface for chatting with Mistral AI's language models.

## Features

- Blazingly fast responses
- Clean, colourised output
- Command history with persistence
- Smart text wrapping for readability
- Secure API key handling via environment variables
- Rich markdown rendering support:
  - Code blocks
  - Lists
  - Bold and italic text
  - Inline code

## Prerequisites

- Rust (latest stable version)
- A Mistral AI API key

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/mistral-small.git
cd mistral-small
```

2. Create a `.env` file in the project root and add your Mistral AI API key:
```bash
MISTRAL_API_KEY=your-api-key-here
```

3. Build and run the project:
```bash
cargo run
```

## Usage

- Type your message and press Enter to send
- Use Up/Down arrow keys to navigate command history
- Type 'exit' to quit the application
- Type 'clear' to clear the screen
- Type 'new' to start a fresh conversation (clears chat history with AI)

## Configuration

The application stores:
- Command history in `~/.mistral_history`
- API key in the `.env` file

## Customisation

The application automatically:
- Adapts to your terminal width
- Maintains consistent formatting
- Preserves chat context

## Author
Samuel Morrison

## Licence

This project is licensed under the MIT Licence - see the [LICENCE](LICENCE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 
