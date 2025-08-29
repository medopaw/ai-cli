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

### Direct Installation from Git (Recommended)

The easiest way to install is directly from GitHub:

```bash
cargo install --git https://github.com/medopaw/ai-cli
```

This will download, compile, and install the latest version from the main branch to `~/.cargo/bin/` which should be in your PATH.

### From Source

If you want to modify the code or contribute:

1. **Clone the repository**:
   ```bash
   git clone git@github.com:medopaw/ai-cli.git
   cd ai-cli
   ```

2. **Install using Cargo**:
   ```bash
   cargo install --path .
   ```

### Alternative: Manual Build

If you prefer to build manually:

```bash
# Build the project
cargo build --release

# Copy binary to a directory in your PATH
cp target/release/ai ~/.local/bin/
# Or add target/release to your PATH
```

### Verify Installation

```bash
ai help
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
You are an expert software engineer that generates concise, 
one-line Git commit messages based on the provided diffs.
Review the provided context and diffs which are about to be committed to a git repo.
Review the diffs carefully.
Generate a one-line commit message for those changes.
The commit message should be structured as follows: <type>: <description>
Use these for <type>: fix, feat, build, chore, ci, docs, style, refactor, perf, test
IMPORTANT: The description must start with a lowercase letter. Never capitalize the first letter of the description.

Ensure the commit message:
- Starts with the appropriate prefix.
- Is in the imperative mood (e.g., "add feature" not "added feature" or "adding feature").
- Does not exceed 72 characters.

Reply only with the one-line commit message, without any additional text, explanations, or line breaks.
Remember: description must start with lowercase letter (e.g., "feat: add new feature", NOT "feat: Add new feature").

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
