use anyhow::{anyhow, Result};
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
            .height("50%".to_string())
            .multi(false)
            .prompt(prompt.to_string())
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
}