use anyhow::{anyhow, Context, Result};
use std::process::Command;

pub struct GitOperations;

impl GitOperations {
    pub fn is_git_repo() -> bool {
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn get_staged_diff() -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "--staged"])
            .output()
            .context("Failed to run git diff --staged")?;

        if !output.status.success() {
            return Err(anyhow!("git diff --staged failed"));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    #[allow(dead_code)]
    pub fn get_unstaged_diff() -> Result<String> {
        let output = Command::new("git")
            .args(["diff"])
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            return Err(anyhow!("git diff failed"));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn get_status() -> Result<String> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .context("Failed to run git status")?;

        if !output.status.success() {
            return Err(anyhow!("git status failed"));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn add_all() -> Result<()> {
        let output = Command::new("git")
            .args(["add", "."])
            .output()
            .context("Failed to run git add .")?;

        if !output.status.success() {
            return Err(anyhow!("git add . failed"));
        }

        Ok(())
    }

    pub fn commit(message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .output()
            .context("Failed to run git commit")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git commit failed: {}", error));
        }

        Ok(())
    }

    pub fn push() -> Result<()> {
        let output = Command::new("git")
            .args(["push"])
            .output()
            .context("Failed to run git push")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git push failed: {}", error));
        }

        Ok(())
    }

    pub fn push_force() -> Result<()> {
        let output = Command::new("git")
            .args(["push", "-f"])
            .output()
            .context("Failed to run git push -f")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git push -f failed: {}", error));
        }

        Ok(())
    }

    pub fn has_remote() -> bool {
        Command::new("git")
            .args(["remote"])
            .output()
            .map(|output| output.status.success() && !output.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn has_upstream() -> bool {
        Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn set_upstream(remote: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["push", "-u", remote, branch])
            .output()
            .context("Failed to set upstream")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to set upstream: {}", error));
        }

        Ok(())
    }

    pub fn get_current_branch() -> Result<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get current branch"));
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub fn add_remote(name: &str, url: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["remote", "add", name, url])
            .output()
            .context("Failed to add remote")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to add remote: {}", error));
        }

        Ok(())
    }

    pub fn get_repository_name() -> Result<String> {
        let current_dir = std::env::current_dir()
            .context("Failed to get current directory")?;
        
        let repo_name = current_dir
            .file_name()
            .context("Failed to get directory name")?
            .to_string_lossy()
            .to_string();

        Ok(repo_name)
    }
}