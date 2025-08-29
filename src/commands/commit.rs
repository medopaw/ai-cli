use anyhow::Result;
use crate::config::Config;
use crate::ai_client::AiClient;
use crate::git_ops::GitOperations;
use crate::utils::Utils;

pub async fn handle_commit(all: bool) -> Result<()> {
    // Check if we're in a git repository
    if !GitOperations::is_git_repo() {
        println!("Error: Not in a git repository");
        return Ok(());
    }

    let config = Config::load()?;
    let (provider_config, command_config) = config.get_git_operations_ai_config()?;
    let client = AiClient::new_with_full_config(provider_config.clone(), command_config.clone(), config.git.clone(), config.clone())?;

    // Handle 'all' flag
    if all {
        println!("Staging all changes...");
        GitOperations::add_all()?;
    }

    // Check for staged changes
    let has_staged = GitOperations::has_staged_changes()?;
    if !has_staged {
        // No staged changes, check if there are unstaged changes
        let has_unstaged = GitOperations::has_unstaged_changes()?;
        if !has_unstaged {
            println!("No changes to commit. Working directory is clean.");
            return Ok(());
        }
        
        // There are unstaged changes but no staged changes
        println!("No staged changes found, but there are unstaged changes.");
        println!("Would you like to stage all changes and commit them?");
        
        if Utils::confirm("Stage all changes and commit?")? {
            println!("Staging all changes...");
            GitOperations::add_all()?;
        } else {
            println!("Commit cancelled.");
            return Ok(());
        }
    }

    // Get staged diff (either from originally staged files or newly staged files)
    let diff = GitOperations::get_staged_diff()?;

    println!("Generating commit message...");
    let commit_message = client.generate_commit_message(&diff).await?;
    
    println!("Commit message: {}", commit_message);
    GitOperations::commit(&commit_message)?;
    println!("✓ Committed successfully!");

    Ok(())
}