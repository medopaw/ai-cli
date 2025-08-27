use anyhow::Result;
use crate::config::Config;
use crate::ai_client::AiClient;
use crate::git_ops::GitOperations;

pub async fn handle_commit(all: bool) -> Result<()> {
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