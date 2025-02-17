# Release Notes

## Version 0.3.0 (2025-02-16)

### Major Features
- Implemented custom welcome message
  - More professional and focused introduction
  - Clear description of capabilities
  - Improved initial user experience
- Added support for Markdown tables with alignment
  - Left, right, and centre column alignment
  - Unicode box-drawing characters for better visual presentation
  - Automatic column width calculation
  - Support for multi-line table cells

### Improvements
- Enhanced markdown rendering capabilities
- Better handling of complex markdown structures
- Improved text wrapping in table cells
- Enhanced documentation
  - Reorganised README.md with clearer structure
  - Added comprehensive example interactions
  - Improved installation and usage instructions
  - Updated feature descriptions
  - Added Mistral AI integration section

### User Experience
- Streamlined initial interaction
- More intuitive command descriptions
- Better organised help information
- Clearer API key requirement documentation

### Technical Updates
- Removed API call for welcome message
- Improved message rendering efficiency
- Updated licence file references
- Enhanced contribution guidelines

## Version 0.2.0 (2025-02-15)

### Major Features
- Added comprehensive syntax highlighting support for 50+ programming languages
- Implemented context-aware language detection from user queries
- Enhanced code block rendering with improved formatting

### Language Support
Added support for multiple programming language categories:
- Systems Programming (Rust, C++, C#, C, Assembly)
- Web Development (JavaScript, TypeScript, HTML, CSS, SCSS)
- Scripting Languages (Python, Ruby, Perl, Lua)
- JVM Languages (Java, Kotlin, Scala, Groovy)
- Mobile Development (Swift, Kotlin Android, Dart/Flutter)
- Data & ML (R, Julia, MATLAB)
- Databases (SQL variants, MongoDB)
- Configuration & Data Formats (JSON, YAML, TOML)
- Modern Languages (Go, Elixir, Haskell, Zig)
- Build & Config (Makefile, CMake, Gradle)
- Version Control (Git-related)
- Markup (Markdown, LaTeX, RST)
- Protocol & Schema (Protobuf, GraphQL)

### Improvements
- Better language detection from user queries
- Improved fallback handling for unknown languages
- Enhanced code block formatting and indentation
- Added support for multiple code blocks with different languages in the same response

### Technical Updates
- Refactored syntax highlighting logic for better maintainability
- Improved error handling for syntax highlighting failures
- Added language hint extraction from user queries
- Updated documentation with new features and examples

## Version 0.1.0 (Initial Release)

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
- coloured 2.1
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
