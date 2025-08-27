use anyhow::Result;
use crate::git_ops::GitOperations;
use crate::utils::Utils;
use super::handle_commit;

pub async fn handle_publish() -> Result<()> {
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