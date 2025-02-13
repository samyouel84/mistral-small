# Release Notes

## v0.1.0 (2024-02-14)

### Initial Release 🚀

#### Features
- Command-line interface for Mistral AI chat interactions
- Real-time chat with the Mistral Small language model
- Intelligent text wrapping and formatting
- Colourised output for improved readability
- Command history with persistent storage
- Secure API key management via `.env`

#### Technical Highlights
- Built with Rust for optimal performance
- Asynchronous API communication using tokio
- Smart terminal handling with rustyline
- Automatic width adaptation for different terminal sizes
- Error handling with proper user feedback

#### User Experience
- Clean, distraction-free interface
- Question context preserved with responses
- Easy navigation with command history
- Simple installation and setup process
- Cross-platform compatibility

#### Dependencies
- tokio 1.36
- reqwest 0.11
- serde 1.0
- rustyline 12.0
- colored 2.1
- textwrap 0.16

### Known Limitations
- Single conversation context only
- No conversation export functionality yet
- No streaming responses in this version

### Coming Soon
- Conversation management
- Response streaming
- Model selection options
- Configuration customisation
- Chat session export 