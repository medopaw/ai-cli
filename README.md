# AI CLI Tool

A personal AI-powered CLI tool built in Rust that integrates with Ollama for intelligent Git operations and project management.

## Features

- 🤖 **AI Integration**: Uses Ollama for AI-powered responses and commit message generation
- 💬 **Interactive Chat**: Multi-turn conversations with AI assistance
- 🚀 **Smart Git Operations**: AI-generated commit messages and intelligent push workflows
- 📦 **Project Publishing**: Support for publishing Rust crates to crates.io
- ⚙️ **Configurable**: TOML-based configuration with sensible defaults

## Commands

### Basic Commands
- `ai help` - Display help information
- `ai ask "question"` - Ask AI a single question
- `ai chat` - Start interactive chat session

### Git Operations
- `ai commit` - Generate AI-powered commit message for staged changes
- `ai commit all` - Stage all changes and commit with AI-generated message
- `ai push` - Intelligent push with conflict resolution
- `ai push force` - Force push changes

### Project Management
- `ai publish` - Publish Rust project to crates.io

### Chat Commands
Within `ai chat`, you can use:
- `/help` - Show chat commands
- `/commit [all]` - Commit changes
- `/push [force]` - Push changes
- `/publish` - Publish project
- `/exit` or `/quit` - Exit chat

## Installation

1. Clone this repository
2. Build with Cargo:
   ```bash
   cargo build --release
   ```
3. Install the binary:
   ```bash
   cp target/release/ai ~/.local/bin/
   # Or add target/release to your PATH
   ```

## Configuration

The tool automatically creates a configuration file at `~/.ai.conf.toml` with default settings:

```toml
[ai]
provider = "ollama"
model = "qwen2.5:7b"
base_url = "http://localhost:11434"

[git]
commit_prompt = """
请为以下 git diff 生成简洁的英文 commit message:
- 遵循约定式提交规范
- 首行不超过50字符
- 如有需要，添加详细描述

Git diff:
{diff}
"""

[history]
enabled = false
```

## Prerequisites

- **Rust** (latest stable)
- **Ollama** running locally with your preferred model
- **Git** for version control operations
- **gh** or **glab** CLI tools (optional, for creating remote repositories)

## Examples

```bash
# Ask AI a programming question
ai ask "How do I implement a binary tree in Rust?"

# Start interactive chat
ai chat

# Generate and apply AI commit message
ai commit all

# Intelligent push with conflict resolution
ai push

# Publish Rust crate
ai publish
```

## Development

The project is organized into modules:
- `ai_client.rs` - AI integration with Ollama
- `git_ops.rs` - Git operations wrapper
- `config.rs` - Configuration management
- `utils.rs` - Utility functions (menus, confirmations)
- `history.rs` - Optional command history (SQLite)

## Architecture

- **AI Provider**: Ollama (extensible to other providers)
- **Configuration**: TOML-based with automatic defaults
- **Git Integration**: Command-line Git wrapper with error handling
- **User Interface**: Command-line with interactive prompts

## License

MIT License

## Contributing

This is a personal tool, but contributions are welcome! Please open an issue or pull request.