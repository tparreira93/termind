# Termind

An interactive terminal with AI agents integration, built in Rust. Termind intelligently interprets commands as either regular shell commands or AI queries, providing a seamless experience for both system administration and AI-assisted tasks.

## Features

- **Intelligent Command Interpretation**: Automatically distinguishes between shell commands and AI queries
- **Local and Remote AI**: Support for both local AI models and API-based AI services
- **Shell Integration**: Executes commands in the system's default shell (zsh, bash, etc.)
- **Real-time UI**: Interactive terminal interface built with Ratatui
- **Cross-platform**: Works on macOS, Linux, and Windows

## Installation

### Prerequisites
- Rust 1.70 or later
- System shell (zsh, bash, PowerShell, etc.)

### Build from Source
```bash
git clone <repository-url>
cd termind
cargo build --release
```

### Run
```bash
cargo run
```

## Usage

Once launched, Termind presents an interactive terminal interface showing your current directory. You can enter:

### Regular Shell Commands
```bash
ls -la
cd /path/to/directory
git status
python script.py
```

### AI Queries
The AI detection is context-aware. Commands that look like questions or requests for assistance are routed to AI:
```bash
how do I find all Python files recursively?
explain this error message
what's the best way to optimize this SQL query?
```

### Command Prefixes
You can also explicitly specify the target:
```bash
shell: ls -la          # Force shell execution
ai: how to use git     # Force AI query
```

## Configuration

Termind supports configuration through a `config.toml` file:

```toml
[ai]
provider = "local"  # or "openai", "anthropic", etc.
api_key = "your-api-key"  # for remote providers
model = "gpt-4"

[shell]
default_shell = "/bin/zsh"  # auto-detected by default

[ui]
theme = "dark"
show_path = true
```

## AI Providers

### Local AI
- Supports local language models via API
- No external dependencies or API keys required
- Privacy-focused approach

### Remote AI Services
- OpenAI GPT models
- Anthropic Claude
- Other compatible APIs

## Development

### Project Structure
```
src/
├── main.rs              # Application entry point
├── shell/               # Shell execution and management
│   ├── mod.rs
│   ├── executor.rs      # Command execution
│   └── parser.rs        # Command parsing
├── ai/                  # AI integration
│   ├── mod.rs
│   ├── detector.rs      # AI vs shell command detection
│   ├── local.rs         # Local AI provider
│   └── remote.rs        # Remote AI providers
├── ui/                  # Terminal UI
│   ├── mod.rs
│   ├── app.rs           # Main application state
│   ├── input.rs         # Input handling
│   └── render.rs        # UI rendering
├── config/              # Configuration management
│   ├── mod.rs
│   └── settings.rs
└── error.rs             # Error types
```

### Running Tests
```bash
cargo test
```

### Running with Debug Logs
```bash
RUST_LOG=debug cargo run
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see LICENSE file for details.
