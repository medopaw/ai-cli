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
                        
                        // Get repository name from current directory
                        let repo_name = match GitOperations::get_repository_name() {
                            Ok(name) => name,
                            Err(e) => {
                                println!("Error: Failed to get repository name: {}", e);
                                return Ok(());
                            }
                        };

                        // Ask if the repository should be private
                        let is_private = Utils::confirm("Make repository private?")?;
                        
                        // Create GitHub repository
                        match Utils::create_github_repository(&repo_name, is_private) {
                            Ok(repo_url) => {
                                println!("✓ GitHub repository created: {}", repo_url);
                                
                                // The gh CLI with --source and --push flags should have already:
                                // 1. Added the remote
                                // 2. Pushed the code
                                // So we can just confirm success and exit
                                println!("✓ Code pushed successfully!");
                                return Ok(());
                            }
                            Err(e) => {
                                println!("Error creating GitHub repository: {}", e);
                                return Ok(());
                            }
                        }
                    }
                    choice if choice.contains("GitLab") => {
                        println!("Creating GitLab repository...");
                        
                        // Get repository name from current directory
                        let repo_name = match GitOperations::get_repository_name() {
                            Ok(name) => name,
                            Err(e) => {
                                println!("Error: Failed to get repository name: {}", e);
                                return Ok(());
                            }
                        };

                        // Ask if the repository should be private
                        let is_private = Utils::confirm("Make repository private?")?;
                        
                        // Create GitLab repository
                        match Utils::create_gitlab_repository(&repo_name, is_private) {
                            Ok(message) => {
                                println!("✓ GitLab repository created: {}", message);
                                
                                // For GitLab, we might need to manually add remote and push
                                // This depends on glab behavior, but for safety let's add the remote
                                if let Ok(username) = Utils::get_gitlab_username() {
                                    let repo_url = format!("git@gitlab.com:{}/{}.git", username, repo_name);
                                    if let Err(e) = GitOperations::add_remote("origin", &repo_url) {
                                        println!("Warning: Failed to add remote: {}", e);
                                    }
                                }
                                
                                // Try to push
                                match GitOperations::push() {
                                    Ok(()) => println!("✓ Code pushed successfully!"),
                                    Err(_) => {
                                        // Try setting upstream and push
                                        if let Ok(branch) = GitOperations::get_current_branch() {
                                            if let Err(e) = GitOperations::set_upstream("origin", &branch) {
                                                println!("Error: Failed to push code: {}", e);
                                            } else {
                                                println!("✓ Code pushed successfully!");
                                            }
                                        }
                                    }
                                }
                                return Ok(());
                            }
                            Err(e) => {
                                println!("Error creating GitLab repository: {}", e);
                                return Ok(());
                            }
                        }
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