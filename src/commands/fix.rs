use anyhow::Result;
use crate::config::Config;
use crate::ai_client::AiClient;
use crate::utils::{Utils, HistoryEntry};

pub async fn handle_fix(user_context: &str) -> Result<()> {
    println!("🔍 Analyzing terminal history for errors...");
    
    // Check zsh configuration for better error tracking
    check_zsh_configuration();
    
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
                let (provider_config, command_config) = config.get_error_analysis_ai_config()?;
                let client = AiClient::new_with_full_config(provider_config.clone(), command_config.clone(), config.git.clone(), config.clone())?;
                
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
    let (provider_config, command_config) = config.get_error_analysis_ai_config()?;
    let client = AiClient::new_with_full_config(provider_config.clone(), command_config.clone(), config.git.clone(), config.clone())?;

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

pub fn extract_commands_from_response(response: &str) -> Option<Vec<String>> {
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

fn check_zsh_configuration() {
    let shell = Utils::get_current_shell().unwrap_or_else(|_| "unknown".to_string());
    
    if shell == "zsh" && !Utils::is_zsh_extended_history_enabled() {
        println!("💡 Tip: Enable zsh EXTENDED_HISTORY for better error tracking:");
        Utils::show_zsh_extended_history_tip();
        println!();
    }
}