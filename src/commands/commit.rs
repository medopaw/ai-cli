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

    // Get provider-specific max diff length
    let (_provider_config, command_config) = config.get_git_operations_ai_config()?;
    let max_diff_length = config.get_max_diff_length_for_provider(&command_config.provider, &command_config.model);

    // Check diff length and decide processing strategy
    let commit_message = if diff.len() > max_diff_length {
        println!("Large diff detected ({} chars). Using intelligent processing...", diff.len());
        
        // Generate overall statistics
        let stats = GitOperations::generate_diff_stats(&diff);
        
        // Segment the diff by files for parallel processing
        let segments = GitOperations::segment_diff_by_files(&diff, max_diff_length);
        
        if segments.is_empty() {
            return Err(anyhow::anyhow!("Failed to segment diff for processing"));
        }

        // Process segments in parallel to get file summaries
        let file_summaries = client.summarize_diff_segments(segments).await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to analyze diff segments: {}\n\n\
                    💡 Suggestions:\n\
                    • Try staging fewer files at once: git add <specific-files>\n\
                    • Ensure your network connection is stable\n\
                    • Check if your AI provider is responding correctly",
                    e
                )
            })?;

        // Generate final commit message based on stats and summaries
        client.generate_final_commit_message(&stats, &file_summaries).await?
    } else {
        // Use original logic for smaller diffs
        println!("Generating commit message...");
        client.generate_commit_message(&diff).await?
    };
    
    println!("Commit message: {}", commit_message);
    GitOperations::commit(&commit_message)?;
    println!("✓ Committed successfully!");

    Ok(())
}