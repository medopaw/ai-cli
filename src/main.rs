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
        "fix" => {
            let user_context = remaining_args.join(" ");
            handle_fix(&user_context).await?;
        }
        "setup" => {
            if remaining_args.contains(&"zsh".to_string()) {
                let advanced = remaining_args.contains(&"--advanced".to_string());
                handle_setup_zsh(advanced).await?;
            } else {
                handle_setup().await?;
            }
        }
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
    println!("    fix        Analyze terminal history and fix the last error");
    println!("    setup      Show setup instructions for better AI CLI experience");
    println!();
    println!("EXAMPLES:");
    println!("    ai ask \"How do I write a Rust function?\"");
    println!("    ai chat");
    println!("    ai commit all");
    println!("    ai push force");
    println!("    ai fix");
    println!("    ai fix \"cargo build failed with linking error\"");
    println!("    ai setup zsh");
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

async fn handle_fix(user_context: &str) -> Result<()> {
    use crate::utils::{Utils, HistoryEntry};
    
    println!("🔍 Analyzing terminal history for errors...");
    
    // Get shell history (last 25 commands to give more context)
    let history = match Utils::get_extended_shell_history(25) {
        Ok(hist) => hist,
        Err(e) => {
            println!("Warning: Could not get extended history ({})", e);
            println!("Trying basic history...");
            
            match Utils::get_shell_history(25) {
                Ok(commands) => commands.into_iter().map(|cmd| HistoryEntry {
                    command: cmd,
                    exit_code: None,
                    timestamp: None,
                }).collect(),
                Err(e) => {
                    eprintln!("Error: Could not get command history: {}", e);
                    return Ok(());
                }
            }
        }
    };

    if history.is_empty() {
        println!("⚠️  No command history found.");
        println!();
        
        // Try to read startup errors from log file
        match Utils::get_recent_startup_errors() {
            Ok(startup_errors) => {
                println!("✅ Found recent startup errors in log file!");
                println!("🔍 Analyzing startup errors automatically...");
                println!();
                
                let startup_context = startup_errors.join("\n");
                let config = Config::load()?;
                let client = AiClient::new(config.ai, config.git)?;
                
                let mut context = String::new();
                context.push_str(&format!("Shell: {}\n", Utils::get_current_shell().unwrap_or_else(|_| "unknown".to_string())));
                context.push_str("Analysis Type: Shell startup errors (auto-detected from log)\n");
                context.push_str("Recent startup errors:\n");
                context.push_str("===================\n");
                context.push_str(&startup_context);
                context.push_str("\nNote: These errors were automatically captured during shell startup.\n");
                
                println!("🧠 Analyzing startup errors and generating solution...");
                let ai_response = client.analyze_and_fix_error(&context, user_context).await?;
                
                println!("\n{}", ai_response);
                
                if let Some(commands) = extract_commands_from_response(&ai_response) {
                    if !commands.is_empty() {
                        println!("\n📋 Copying commands to clipboard...");
                        let commands_text = commands.join("\n");
                        
                        match Utils::copy_to_clipboard(&commands_text) {
                            Ok(()) => {
                                println!("✅ Commands copied to clipboard!");
                                println!("💡 You can now paste and execute them in your terminal.");
                            }
                            Err(e) => {
                                println!("❌ Failed to copy to clipboard: {}", e);
                                println!("💡 Here are the commands to run manually:");
                                println!("```bash");
                                for cmd in &commands {
                                    println!("{}", cmd);
                                }
                                println!("```");
                            }
                        }
                    }
                }
                
                return Ok(());
            }
            Err(_) => {
                println!("This might be because:");
                println!("• This is a fresh shell session");
                println!("• Shell history is disabled");
                println!("• You're seeing startup errors that aren't in command history");
                println!();
                println!("💡 Options to get startup error analysis:");
                println!("1. Copy and paste the error: ai fix \"error message here\"");
                println!("2. Enable auto-capture: ai setup zsh --advanced");
                println!();
                return Ok(());
            }
        }
    }

    // Check shell support and provide helpful tips
    let shell = Utils::get_current_shell().unwrap_or_else(|_| "unknown".to_string());
    let mut show_zsh_tip = false;
    
    if shell == "zsh" {
        if !Utils::is_zsh_extended_history_enabled() {
            show_zsh_tip = true;
            println!("ℹ️  Note: zsh EXTENDED_HISTORY is not enabled.");
            println!("   This limits error detection accuracy. Run 'ai setup zsh' for help.");
        } else {
            println!("✅ zsh EXTENDED_HISTORY is enabled - excellent for error tracking!");
        }
    } else if !Utils::shell_supports_exit_codes() {
        println!("ℹ️  Note: Your shell ({}) may not provide exit code information.", shell);
        println!("   For better error detection, consider switching to zsh: 'ai setup zsh'");
    }

    // Check if this might be a fresh session with startup errors
    let is_likely_startup_error = history.len() < 5 && 
        !user_context.is_empty() && 
        (user_context.contains("warning:") || 
         user_context.contains("error:") || 
         user_context.contains("lock") ||
         user_context.contains("permission denied") ||
         user_context.contains("command not found"));

    // Find the last failed command (or assume last command if no clear failure)
    let failed_cmd_index = Utils::find_last_failed_command(&history)
        .unwrap_or_else(|| history.len().saturating_sub(1));

    // Build context for AI analysis
    let mut context = String::new();
    context.push_str(&format!("Shell: {}\n", shell));
    
    if is_likely_startup_error {
        context.push_str("Analysis Type: Shell startup error (likely from .zshrc/.bashrc)\n");
        context.push_str("User reported error/warning: ");
        context.push_str(user_context);
        context.push_str("\n\n");
        context.push_str("Command History (limited - fresh session):\n");
        context.push_str("==========================================\n");
        if history.is_empty() {
            context.push_str("No commands executed yet\n");
        } else {
            for (i, entry) in history.iter().enumerate() {
                context.push_str(&format!("{}. {}\n", i + 1, entry.command));
            }
        }
        context.push_str("\nNote: This appears to be a shell startup error, not a command execution error.\n");
    } else {
        context.push_str(&format!("Total commands in context: {}\n", history.len()));
        context.push_str(&format!("Suspected failed command at index: {}\n\n", failed_cmd_index + 1));
        
        context.push_str("Command History:\n");
        context.push_str("================\n");
        
        for (i, entry) in history.iter().enumerate() {
            let marker = if i == failed_cmd_index { " ❌ " } else { "    " };
            let exit_info = match entry.exit_code {
                Some(code) => format!(" (exit: {})", code),
                None => String::new(),
            };
            context.push_str(&format!("{}{}. {}{}\n", marker, i + 1, entry.command, exit_info));
        }
        
        context.push_str("\nNote: ❌ indicates the suspected failed command\n");
    }

    // Load AI configuration and analyze
    println!("🤖 Loading AI configuration...");
    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;

    println!("🧠 Analyzing error and generating solution...");
    let ai_response = client.analyze_and_fix_error(&context, user_context).await?;

    // Display the analysis
    println!("\n{}", ai_response);

    // Extract commands from the response
    if let Some(commands) = extract_commands_from_response(&ai_response) {
        if !commands.is_empty() {
            println!("\n📋 Copying commands to clipboard...");
            let commands_text = commands.join("\n");
            
            match Utils::copy_to_clipboard(&commands_text) {
                Ok(()) => {
                    println!("✅ Commands copied to clipboard!");
                    println!("💡 You can now paste and execute them in your terminal.");
                }
                Err(e) => {
                    println!("❌ Failed to copy to clipboard: {}", e);
                    println!("💡 Here are the commands to run manually:");
                    println!("```bash");
                    for cmd in &commands {
                        println!("{}", cmd);
                    }
                    println!("```");
                }
            }
        }
    }

    // Show zsh setup tip if applicable
    if show_zsh_tip {
        println!();
        Utils::show_zsh_extended_history_tip();
    }

    Ok(())
}

fn extract_commands_from_response(response: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = response.lines().collect();
    let mut in_code_block = false;
    let mut commands = Vec::new();
    
    for line in lines {
        if line.trim().starts_with("```bash") || line.trim().starts_with("```sh") || line.trim() == "```" {
            in_code_block = !in_code_block;
            continue;
        }
        
        if in_code_block {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                commands.push(trimmed.to_string());
            }
        }
    }
    
    if commands.is_empty() {
        None
    } else {
        Some(commands)
    }
}

async fn handle_setup() -> Result<()> {
    println!("🛠️  AI CLI Setup Guide");
    println!();
    println!("Available setup options:");
    println!("  ai setup zsh     - Configure zsh for better error tracking");
    println!();
    println!("For more specific setup instructions, run:");
    println!("  ai setup <option>");
    Ok(())
}

async fn handle_setup_zsh(advanced: bool) -> Result<()> {
    use crate::utils::Utils;
    
    println!("🐚 Setting up zsh for optimal AI CLI experience");
    println!();
    
    let shell = Utils::get_current_shell().unwrap_or_else(|_| "unknown".to_string());
    if shell != "zsh" {
        println!("⚠️  Warning: You are currently using {} shell, not zsh.", shell);
        println!("   The following instructions are specifically for zsh.");
        println!();
    }
    
    println!("🔧 Step 1: Check current zsh configuration");
    if Utils::is_zsh_extended_history_enabled() {
        println!("✅ zsh EXTENDED_HISTORY is already enabled!");
        println!("   Your setup is optimal for ai fix command.");
    } else {
        println!("❌ zsh EXTENDED_HISTORY is not enabled.");
        println!();
        
        println!("🔧 Step 2: Add configuration to ~/.zshrc");
        Utils::show_zsh_extended_history_tip();
        
        println!("🔧 Step 3: Apply the changes");
        println!("Run this command to reload your zsh configuration:");
        println!("   source ~/.zshrc");
        println!();
        
        println!("🔧 Step 4: Verify the setup");
        println!("After reloading, run this to verify:");
        println!("   ai setup zsh");
    }
    
    if advanced {
        println!();
        Utils::show_error_capture_setup();
    } else {
        println!("📚 Additional zsh optimizations you might want:");
        println!("┌─────────────────────────────────────────────────┐");
        println!("│ # More history options                          │");
        println!("│ HISTSIZE=10000                                  │");
        println!("│ SAVEHIST=10000                                  │");
        println!("│ setopt HIST_FIND_NO_DUPS                        │");
        println!("│ setopt HIST_IGNORE_SPACE                        │");
        println!("│ # setopt SHARE_HISTORY  # Optional: cross-term  │");
        println!("└─────────────────────────────────────────────────┘");
        
        println!();
        println!("🔥 For automatic startup error capture, run:");
        println!("   ai setup zsh --advanced");
    }
    
    Ok(())
}
