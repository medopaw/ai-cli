use anyhow::{anyhow, Context, Result};
use skim::prelude::*;
use std::io::Cursor;
use std::process::Command;

pub struct Utils;

impl Utils {
    /// Check if a command line tool is available
    pub fn is_command_available(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Show a selection menu using skim
    pub fn select_option(options: &[&str], prompt: &str) -> Result<Option<String>> {
        if options.is_empty() {
            return Ok(None);
        }

        let options_str = options.join("\n");
        let item_reader = SkimItemReader::default();
        let items = item_reader.of_bufread(Cursor::new(options_str));

        let skim_options = SkimOptionsBuilder::default()
            .multi(false)
            .prompt(prompt.to_string())
            .reverse(true)
            .no_height(true)
            .no_clear(true)
            .build()
            .map_err(|e| anyhow!("Failed to build skim options: {}", e))?;

        let selected_items = Skim::run_with(&skim_options, Some(items));
        
        match selected_items {
            Some(out) if !out.is_abort => {
                if let Some(item) = out.selected_items.first() {
                    Ok(Some(item.output().to_string()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Get current working directory as string  
    #[allow(dead_code)]
    pub fn current_dir() -> Result<String> {
        Ok(std::env::current_dir()
            .map_err(|e| anyhow!("Failed to get current directory: {}", e))?
            .to_string_lossy()
            .to_string())
    }

    /// Check if current directory is a Rust project
    pub fn is_rust_project() -> bool {
        std::path::Path::new("Cargo.toml").exists()
    }

    /// Get project type
    pub fn detect_project_type() -> Option<String> {
        if Self::is_rust_project() {
            Some("rust".to_string())
        } else {
            None
        }
    }

    /// Confirm action with user
    pub fn confirm(message: &str) -> Result<bool> {
        println!("{} (y/N)", message);
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        let input = input.trim().to_lowercase();
        Ok(matches!(input.as_str(), "y" | "yes"))
    }

    /// Get shell history commands
    pub fn get_shell_history(limit: usize) -> Result<Vec<String>> {
        // Try to read history from file directly
        if let Ok(home) = std::env::var("HOME") {
            let shell = Self::get_current_shell().unwrap_or_else(|_| "bash".to_string());
            let history_files = match shell.as_str() {
                "zsh" => vec![format!("{}/.zsh_history", home), format!("{}/.zhistory", home)],
                "bash" => vec![format!("{}/.bash_history", home)],
                "fish" => vec![format!("{}/.local/share/fish/fish_history", home)],
                _ => vec![
                    format!("{}/.bash_history", home),
                    format!("{}/.zsh_history", home),
                ],
            };

            for hist_file in &history_files {
                if let Ok(content) = std::fs::read_to_string(hist_file) {
                    let mut commands: Vec<String> = content
                        .lines()
                        .filter_map(|line| {
                            let line = line.trim();
                            if line.is_empty() {
                                return None;
                            }
                            
                            // Handle zsh extended format: ": 1234567890:0;command"
                            if line.starts_with(':') {
                                if let Some((_, cmd)) = line.split_once(';') {
                                    return Some(cmd.to_string());
                                }
                            }
                            
                            // Handle fish format: "- cmd: command"
                            if line.starts_with("- cmd: ") {
                                return Some(line[7..].to_string());
                            }
                            
                            // Regular history line
                            if !line.starts_with('#') && !line.starts_with(':') {
                                Some(line.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    // Take the last `limit` commands
                    if commands.len() > limit {
                        commands = commands.split_off(commands.len() - limit);
                    }

                    return Ok(commands);
                }
            }
        }

        // Fallback: try fc command (works in many shells)
        if let Ok(output) = Command::new("sh").arg("-c").arg("fc -l -50").output() {
            if output.status.success() {
                let history_text = String::from_utf8_lossy(&output.stdout);
                let mut commands: Vec<String> = history_text
                    .lines()
                    .filter_map(|line| {
                        // Parse fc output format: " 1234  command"
                        if let Some(pos) = line.find(char::is_alphabetic) {
                            Some(line[pos..].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                if commands.len() > limit {
                    commands = commands.split_off(commands.len() - limit);
                }

                return Ok(commands);
            }
        }

        Err(anyhow!("Could not read shell history. Try ensuring your shell saves history to a file."))
    }

    /// Detect the current shell
    pub fn get_current_shell() -> Result<String> {
        if let Ok(shell) = std::env::var("SHELL") {
            Ok(shell.split('/').last().unwrap_or("unknown").to_string())
        } else {
            Err(anyhow!("Could not determine current shell"))
        }
    }

    /// Check if shell supports command history with exit codes
    pub fn shell_supports_exit_codes() -> bool {
        // For now, we'll assume zsh and bash support this with proper configuration
        if let Ok(shell) = Self::get_current_shell() {
            matches!(shell.as_str(), "zsh" | "bash")
        } else {
            false
        }
    }

    /// Check if zsh has EXTENDED_HISTORY enabled
    pub fn is_zsh_extended_history_enabled() -> bool {
        if let Ok(home) = std::env::var("HOME") {
            let hist_file = format!("{}/.zsh_history", home);
            // Use read() instead of read_to_string() to handle non-UTF8 bytes
            if let Ok(bytes) = std::fs::read(&hist_file) {
                let content = String::from_utf8_lossy(&bytes);
                // Check if history contains extended format entries
                for line in content.lines().take(50) { // Check first 50 lines
                    if line.starts_with(':') && line.contains(';') {
                        // Found extended format: ": timestamp:duration;command"
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Show zsh EXTENDED_HISTORY setup instructions
    pub fn show_zsh_extended_history_tip() {
        println!("💡 Tip: For better error detection in zsh, enable EXTENDED_HISTORY");
        println!();
        println!("Add this to your ~/.zshrc file:");
        println!("┌─────────────────────────────────────────────────────┐");
        println!("│ # Enable extended history for better error tracking │");
        println!("│ setopt EXTENDED_HISTORY                             │");
        println!("│ setopt HIST_EXPIRE_DUPS_FIRST                       │");
        println!("│ setopt HIST_IGNORE_DUPS                             │");
        println!("└─────────────────────────────────────────────────────┘");
        println!();
        println!("Then run: source ~/.zshrc");
        println!("This will provide timestamps and better error context for ai fix.");
        println!();
    }

    /// Show advanced error capture setup
    #[allow(dead_code)]
    pub fn show_error_capture_setup() {
        println!("🔥 Advanced: Auto-capture shell startup errors");
        println!();
        println!("To automatically capture startup errors, add this to the");
        println!("TOP of your ~/.zshrc file (before any other commands):");
        println!("┌─────────────────────────────────────────────────────┐");
        println!("│ # Auto-capture startup errors for ai fix           │");
        println!("│ exec 2> >(tee -a ~/.zsh_startup_errors.log >&2)    │");
        println!("└─────────────────────────────────────────────────────┘");
        println!();
        println!("This will log all startup errors to ~/.zsh_startup_errors.log");
        println!("Then ai fix can automatically read and analyze them!");
        println!();
        println!("⚠️  Note: This is advanced setup. Only add if you understand");
        println!("   shell redirection. You can remove it anytime if needed.");
        println!();
    }

    /// Try to read startup errors from log file
    pub fn get_recent_startup_errors() -> Result<Vec<String>> {
        if let Ok(home) = std::env::var("HOME") {
            let error_log = format!("{}/.zsh_startup_errors.log", home);
            if let Ok(content) = std::fs::read_to_string(&error_log) {
                let recent_errors: Vec<String> = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .rev() // 最新的错误在前
                    .take(20) // 只取最近20行
                    .map(|s| s.to_string())
                    .collect();
                
                if !recent_errors.is_empty() {
                    return Ok(recent_errors);
                }
            }
        }
        Err(anyhow!("No startup error log found"))
    }

    /// Get extended shell history (with exit codes if available)
    pub fn get_extended_shell_history(limit: usize) -> Result<Vec<HistoryEntry>> {
        let shell = Self::get_current_shell()?;
        
        match shell.as_str() {
            "zsh" => Self::get_zsh_extended_history(limit),
            "bash" => Self::get_bash_extended_history(limit),
            _ => {
                // Fallback to basic history
                let commands = Self::get_shell_history(limit)?;
                Ok(commands.into_iter().map(|cmd| HistoryEntry {
                    command: cmd,
                    exit_code: None,
                    timestamp: None,
                }).collect())
            }
        }
    }

    /// Get zsh extended history (requires EXTENDED_HISTORY option)
    fn get_zsh_extended_history(limit: usize) -> Result<Vec<HistoryEntry>> {
        // First try the history file directly
        if let Ok(home) = std::env::var("HOME") {
            let hist_file = format!("{}/.zsh_history", home);
            // Use read() instead of read_to_string() to handle non-UTF8 bytes
            if let Ok(bytes) = std::fs::read(&hist_file) {
                let content = String::from_utf8_lossy(&bytes);
                let mut entries = Vec::new();
                
                for line in content.lines() {
                    if line.starts_with(':') {
                        // Extended format: ": 1234567890:0;command"
                        if let Some((_, rest)) = line.split_once(';') {
                            // Parse timestamp and exit code if available
                            entries.push(HistoryEntry {
                                command: rest.to_string(),
                                exit_code: None, // Would need more parsing
                                timestamp: None,
                            });
                        }
                    } else if !line.trim().is_empty() {
                        entries.push(HistoryEntry {
                            command: line.to_string(),
                            exit_code: None,
                            timestamp: None,
                        });
                    }
                }
                
                if entries.len() > limit {
                    entries = entries.split_off(entries.len() - limit);
                }
                
                return Ok(entries);
            }
        }
        
        // Fallback to basic history
        let commands = Self::get_shell_history(limit)?;
        Ok(commands.into_iter().map(|cmd| HistoryEntry {
            command: cmd,
            exit_code: None,
            timestamp: None,
        }).collect())
    }

    /// Get bash extended history
    fn get_bash_extended_history(limit: usize) -> Result<Vec<HistoryEntry>> {
        // Bash doesn't store exit codes in history by default
        // Fallback to basic history
        let commands = Self::get_shell_history(limit)?;
        Ok(commands.into_iter().map(|cmd| HistoryEntry {
            command: cmd,
            exit_code: None,
            timestamp: None,
        }).collect())
    }

    /// Find the last failed command in history
    pub fn find_last_failed_command(history: &[HistoryEntry]) -> Option<usize> {
        // This is a heuristic approach since we don't always have exit codes
        // Look for common patterns that indicate failure
        for (i, entry) in history.iter().enumerate().rev() {
            // If we have exit code information
            if let Some(code) = entry.exit_code {
                if code != 0 {
                    return Some(i);
                }
            }
            
            // Heuristic: look for commands that commonly fail
            let cmd = entry.command.trim().to_lowercase();
            if cmd.starts_with("cargo") || 
               cmd.starts_with("npm") || 
               cmd.starts_with("git") ||
               cmd.starts_with("make") ||
               cmd.starts_with("docker") {
                // For now, assume the most recent command might be the failed one
                // This is not perfect but better than nothing
                if i == history.len() - 1 {
                    return Some(i);
                }
            }
        }
        
        None
    }

    /// Copy text to clipboard
    pub fn copy_to_clipboard(text: &str) -> Result<()> {
        // Try different clipboard tools
        let clipboard_tools = [
            ("pbcopy", vec![]), // macOS
            ("xclip", vec!["-selection", "clipboard"]), // Linux X11
            ("wl-copy", vec![]), // Linux Wayland
        ];

        for (tool, args) in &clipboard_tools {
            if Self::is_command_available(tool) {
                let mut cmd = Command::new(tool);
                for arg in args {
                    cmd.arg(arg);
                }
                
                let mut child = cmd
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .context(format!("Failed to spawn {}", tool))?;

                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    stdin.write_all(text.as_bytes())?;
                }

                let output = child.wait()?;
                if output.success() {
                    return Ok(());
                }
            }
        }

        Err(anyhow!("No supported clipboard tool found. Install pbcopy (macOS), xclip (Linux X11), or wl-copy (Linux Wayland)"))
    }

    /// Create a GitHub repository using gh CLI
    pub fn create_github_repository(repo_name: &str, is_private: bool) -> Result<String> {
        if !Self::is_command_available("gh") {
            return Err(anyhow!("GitHub CLI (gh) is not installed. Please install it: brew install gh"));
        }

        // Check if user is authenticated
        let auth_output = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .context("Failed to check GitHub authentication status")?;

        if !auth_output.status.success() {
            return Err(anyhow!("Not authenticated with GitHub. Run: gh auth login"));
        }

        // Create the repository
        let mut args = vec!["repo", "create", repo_name];
        
        if is_private {
            args.push("--private");
        } else {
            args.push("--public");
        }
        
        // Add other useful flags
        args.extend(&["--source=.", "--push"]);

        let output = Command::new("gh")
            .args(&args)
            .output()
            .context("Failed to create GitHub repository")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create GitHub repository: {}", error));
        }

        // Extract the repository URL from stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // The gh command typically outputs the repository URL
        for line in stdout.lines() {
            if line.contains("github.com") && (line.starts_with("https://") || line.contains("git@")) {
                return Ok(line.trim().to_string());
            }
        }

        // Fallback: construct the URL manually
        let auth_user_output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .output()
            .context("Failed to get GitHub username")?;

        if auth_user_output.status.success() {
            let username = String::from_utf8_lossy(&auth_user_output.stdout).trim().to_string();
            Ok(format!("https://github.com/{}/{}", username, repo_name))
        } else {
            Ok(format!("Repository '{}' created successfully", repo_name))
        }
    }

    /// Create a GitLab repository using glab CLI
    pub fn create_gitlab_repository(repo_name: &str, is_private: bool) -> Result<String> {
        if !Self::is_command_available("glab") {
            return Err(anyhow!("GitLab CLI (glab) is not installed. Please install it: brew install glab"));
        }

        // Check if user is authenticated
        let auth_output = Command::new("glab")
            .args(["auth", "status"])
            .output()
            .context("Failed to check GitLab authentication status")?;

        if !auth_output.status.success() {
            return Err(anyhow!("Not authenticated with GitLab. Run: glab auth login"));
        }

        // Create the repository
        let mut args = vec!["repo", "create", repo_name];
        
        if is_private {
            args.push("--private");
        } else {
            args.push("--public");
        }

        let output = Command::new("glab")
            .args(&args)
            .output()
            .context("Failed to create GitLab repository")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create GitLab repository: {}", error));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    /// Get GitLab username using glab CLI
    pub fn get_gitlab_username() -> Result<String> {
        let output = Command::new("glab")
            .args(["api", "user", "--jq", ".username"])
            .output()
            .context("Failed to get GitLab username")?;

        if output.status.success() {
            let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !username.is_empty() {
                Ok(username)
            } else {
                Err(anyhow!("Empty username returned"))
            }
        } else {
            Err(anyhow!("Failed to retrieve GitLab username"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub exit_code: Option<i32>,
    #[allow(dead_code)]
    pub timestamp: Option<String>,
}