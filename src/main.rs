mod cli;
mod config;
mod ai_client;
mod git_ops;
mod history;
mod utils;

// use clap::Parser;
// use cli::{Cli, Commands};
use config::Config;
use ai_client::AiClient;
use git_ops::GitOperations;
use utils::Utils;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    
    // Handle no arguments or unrecognized commands
    if args.len() == 1 {
        show_help();
        return Ok(());
    }

    // Parse command manually to handle unrecognized commands
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let remaining_args: Vec<String> = args.iter().skip(2).map(|s| s.clone()).collect();

    match subcommand {
        "help" => show_help(),
        "ask" => {
            if remaining_args.is_empty() {
                eprintln!("Error: 'ask' command requires a question");
                show_help();
                return Ok(());
            }
            let question = remaining_args.join(" ");
            handle_ask(&question).await?;
        }
        "chat" => handle_chat().await?,
        "commit" => {
            let all = remaining_args.contains(&"all".to_string());
            handle_commit(all).await?;
        }
        "push" => {
            let force = remaining_args.contains(&"force".to_string());
            handle_push(force).await?;
        }
        "publish" => handle_publish().await?,
        _ => {
            eprintln!("Error: Unknown command '{}'", subcommand);
            show_help();
        }
    }

    Ok(())
}

fn show_help() {
    println!("ai - Personal AI CLI tool");
    println!();
    println!("USAGE:");
    println!("    ai <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    help       Display this help message");
    println!("    ask        Ask AI a single question");
    println!("    chat       Start interactive chat session");
    println!("    commit     Commit changes with AI-generated message");
    println!("    push       Push changes to remote repository");
    println!("    publish    Publish project to appropriate registry");
    println!();
    println!("EXAMPLES:");
    println!("    ai ask \"How do I write a Rust function?\"");
    println!("    ai chat");
    println!("    ai commit all");
    println!("    ai push force");
}

async fn handle_ask(question: &str) -> Result<()> {
    println!("Loading configuration...");
    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;
    
    println!("Asking AI: {}", question);
    let response = client.ask(question).await?;
    println!("{}", response);
    
    Ok(())
}

async fn handle_chat() -> Result<()> {
    println!("Starting chat session... (type /exit or /quit to leave)");
    println!("Available commands: /help, /commit, /push, /publish, /exit, /quit");
    println!();
    
    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;
    
    let mut conversation: Vec<ai_client::ChatMessage> = Vec::new();
    
    loop {
        print!("You: ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Handle slash commands
        if input.starts_with('/') {
            match input {
                "/exit" | "/quit" => {
                    println!("Goodbye!");
                    break;
                }
                "/help" => {
                    show_chat_help();
                    continue;
                }
                "/commit" => {
                    if let Err(e) = handle_commit(false).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/commit all" => {
                    if let Err(e) = handle_commit(true).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/push" => {
                    if let Err(e) = handle_push(false).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/push force" => {
                    if let Err(e) = handle_push(true).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/publish" => {
                    if let Err(e) = handle_publish().await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                _ => {
                    println!("Unknown command: {}. Type /help for available commands.", input);
                    continue;
                }
            }
        }
        
        // Add user message to conversation
        conversation.push(ai_client::ChatMessage::user(input));
        
        // Get AI response
        match client.chat(&conversation).await {
            Ok(response) => {
                println!("AI: {}", response);
                // Add AI response to conversation
                conversation.push(ai_client::ChatMessage::assistant(response));
            }
            Err(e) => {
                println!("Error getting AI response: {}", e);
            }
        }
    }
    
    Ok(())
}

fn show_chat_help() {
    println!("Chat Commands:");
    println!("  /help          Show this help message");
    println!("  /commit        Commit changes with AI-generated message");
    println!("  /commit all    Stage all changes and commit with AI-generated message");
    println!("  /push          Push changes to remote repository");
    println!("  /push force    Force push changes to remote repository");
    println!("  /publish       Publish project to appropriate registry");
    println!("  /exit, /quit   Exit the chat session");
}

async fn handle_commit(all: bool) -> Result<()> {
    // Check if we're in a git repository
    if !GitOperations::is_git_repo() {
        println!("Error: Not in a git repository");
        return Ok(());
    }

    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;

    // Handle 'all' flag
    if all {
        println!("Staging all changes...");
        GitOperations::add_all()?;
    }

    // Get staged diff
    let diff = GitOperations::get_staged_diff()?;
    if diff.trim().is_empty() {
        println!("No staged changes to commit");
        return Ok(());
    }

    println!("Generating commit message...");
    let commit_message = client.generate_commit_message(&diff).await?;
    
    println!("Commit message: {}", commit_message);
    GitOperations::commit(&commit_message)?;
    println!("✓ Committed successfully!");

    Ok(())
}

async fn handle_push(force: bool) -> Result<()> {
    // Check if we're in a git repository
    if !GitOperations::is_git_repo() {
        println!("Error: Not in a git repository");
        return Ok(());
    }

    // Check for uncommitted changes
    let status = GitOperations::get_status()?;
    if !status.trim().is_empty() {
        println!("You have uncommitted changes:");
        println!("{}", status);
        
        let options = if GitOperations::get_staged_diff()?.trim().is_empty() {
            vec![
                "Commit all changes and push",
                "Push anyway (ignore uncommitted changes)",
                "Cancel"
            ]
        } else {
            vec![
                "Commit staged changes and push",
                "Commit all changes and push", 
                "Push anyway (ignore uncommitted changes)",
                "Cancel"
            ]
        };
        
        match Utils::select_option(&options, "What would you like to do?")? {
            Some(choice) => {
                match choice.as_str() {
                    choice if choice.contains("Commit staged") => {
                        handle_commit(false).await?;
                    }
                    choice if choice.contains("Commit all") => {
                        handle_commit(true).await?;
                    }
                    choice if choice.contains("Push anyway") => {
                        // Continue with push
                    }
                    _ => {
                        println!("Push cancelled");
                        return Ok(());
                    }
                }
            }
            None => {
                println!("Push cancelled");
                return Ok(());
            }
        }
    }

    // Check if remote exists
    if !GitOperations::has_remote() {
        println!("No remote repository configured.");
        let mut available_tools = Vec::new();
        
        if Utils::is_command_available("gh") {
            available_tools.push("Create GitHub repository (gh)");
        }
        if Utils::is_command_available("glab") {
            available_tools.push("Create GitLab repository (glab)");
        }
        available_tools.push("Cancel");

        if available_tools.len() == 1 {
            println!("Please install 'gh' (GitHub CLI) or 'glab' (GitLab CLI) to create a remote repository:");
            println!("  brew install gh");
            println!("  brew install glab");
            return Ok(());
        }

        match Utils::select_option(&available_tools, "Create remote repository?")? {
            Some(choice) => {
                match choice.as_str() {
                    choice if choice.contains("GitHub") => {
                        println!("Creating GitHub repository...");
                        // TODO: Implement gh repo create
                        println!("GitHub repository creation not yet implemented");
                        return Ok(());
                    }
                    choice if choice.contains("GitLab") => {
                        println!("Creating GitLab repository...");
                        // TODO: Implement glab repo create
                        println!("GitLab repository creation not yet implemented");
                        return Ok(());
                    }
                    _ => {
                        println!("Push cancelled");
                        return Ok(());
                    }
                }
            }
            None => {
                println!("Push cancelled");
                return Ok(());
            }
        }
    }

    // Perform the push
    let push_result = if force {
        GitOperations::push_force()
    } else {
        GitOperations::push()
    };

    match push_result {
        Ok(()) => {
            println!("✓ Pushed successfully!");
        }
        Err(e) => {
            println!("Push failed: {}", e);
            
            // Try setting upstream if no upstream is configured
            if !GitOperations::has_upstream() {
                if Utils::confirm("Set upstream branch and push?")? {
                    let branch = GitOperations::get_current_branch()?;
                    GitOperations::set_upstream("origin", &branch)?;
                    println!("✓ Upstream set and pushed successfully!");
                }
            }
        }
    }

    Ok(())
}

async fn handle_publish() -> Result<()> {
    // Detect project type
    let project_type = Utils::detect_project_type();
    
    let project_type = match project_type {
        Some(ptype) => ptype,
        None => {
            // Ask user to select project type
            let options = vec!["rust", "Cancel"];
            match Utils::select_option(&options, "Select project type:")? {
                Some(choice) if choice != "Cancel" => choice,
                _ => {
                    println!("Publish cancelled");
                    return Ok(());
                }
            }
        }
    };

    match project_type.as_str() {
        "rust" => {
            println!("Publishing Rust crate to crates.io...");
            
            // Check for uncommitted changes
            if GitOperations::is_git_repo() {
                let status = GitOperations::get_status()?;
                if !status.trim().is_empty() {
                    if Utils::confirm("You have uncommitted changes. Commit them first?")? {
                        handle_commit(true).await?;
                    }
                }
            }

            // Check cargo login
            println!("Make sure you're logged into crates.io:");
            println!("  cargo login");
            
            if Utils::confirm("Proceed with cargo publish?")? {
                use std::process::Command;
                let output = Command::new("cargo")
                    .args(["publish"])
                    .output()?;
                    
                if output.status.success() {
                    println!("✓ Published successfully to crates.io!");
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    println!("Publish failed: {}", error);
                }
            } else {
                println!("Publish cancelled");
            }
        }
        _ => {
            println!("Project type '{}' not supported yet", project_type);
        }
    }

    Ok(())
}
