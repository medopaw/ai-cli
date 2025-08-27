use anyhow::Result;
use crate::utils::Utils;

pub async fn handle_setup() -> Result<()> {
    println!("🛠️  AI CLI Setup Guide");
    println!();
    println!("Available setup options:");
    println!("  ai setup zsh     - Configure zsh for better error tracking");
    println!();
    println!("For more specific setup instructions, run:");
    println!("  ai setup <option>");
    Ok(())
}

pub async fn handle_setup_zsh(advanced: bool) -> Result<()> {
    println!("🐚 Setting up zsh for optimal AI CLI experience");
    println!();
    
    let shell = Utils::get_current_shell().unwrap_or_else(|_| "unknown".to_string());
    if shell != "zsh" {
        println!("⚠️  Warning: You are currently using {} shell, not zsh.", shell);
        println!("   The following instructions are specifically for zsh.");
        println!();
    }
    
    println!("🔧 Step 1: Check current zsh configuration");
    if Utils::is_zsh_extended_history_enabled() {
        println!("✅ zsh EXTENDED_HISTORY is already enabled!");
        println!("   Your setup is optimal for ai fix command.");
    } else {
        println!("❌ zsh EXTENDED_HISTORY is not enabled.");
        println!();
        
        println!("🔧 Step 2: Add configuration to ~/.zshrc");
        Utils::show_zsh_extended_history_tip();
        
        println!("🔧 Step 3: Apply the changes");
        println!("Run this command to reload your zsh configuration:");
        println!("   source ~/.zshrc");
        println!();
        
        println!("🔧 Step 4: Verify the setup");
        println!("After reloading, run this to verify:");
        println!("   ai setup zsh");
    }
    
    if advanced {
        println!();
        Utils::show_error_capture_setup();
    } else {
        println!("📚 Additional zsh optimizations you might want:");
        println!("┌─────────────────────────────────────────────────────┐");
        println!("│ # More history options                          │");
        println!("│ HISTSIZE=10000                                  │");
        println!("│ SAVEHIST=10000                                  │");
        println!("│ setopt HIST_FIND_NO_DUPS                        │");
        println!("│ setopt HIST_IGNORE_SPACE                        │");
        println!("│ # setopt SHARE_HISTORY  # Optional: cross-term  │");
        println!("└─────────────────────────────────────────────────────┘");
        
        println!();
        println!("🔥 For automatic startup error capture, run:");
        println!("   ai setup zsh --advanced");
    }
    
    Ok(())
}