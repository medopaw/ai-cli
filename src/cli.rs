use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ai")]
#[command(about = "Personal AI CLI tool for chat, git operations, and project publishing")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display help information
    Help,
    /// Ask AI a single question
    Ask {
        /// The question to ask
        question: String,
    },
    /// Start interactive chat session
    Chat,
    /// Commit changes with AI-generated message
    Commit {
        /// Stage all files before committing
        all: bool,
    },
    /// Push changes to remote repository
    Push {
        /// Force push changes
        force: bool,
    },
    /// Publish project to appropriate registry
    Publish,
}

impl Commands {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "help" => Some(Commands::Help),
            "ask" => None, // Requires argument
            "chat" => Some(Commands::Chat),
            "commit" => Some(Commands::Commit { all: false }),
            "push" => Some(Commands::Push { force: false }),
            "publish" => Some(Commands::Publish),
            _ => None,
        }
    }
}