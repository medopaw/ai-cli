use anyhow::Result;
use crate::git_ops::GitOperations;
use crate::utils::Utils;
use super::handle_commit;

pub async fn handle_push(force: bool) -> Result<()> {
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