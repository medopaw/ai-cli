use anyhow::Result;
use crate::config::Config;
use crate::ai_client::{AiClient, ChatMessage};
use super::{handle_commit, handle_push, handle_publish};

pub async fn handle_chat() -> Result<()> {
    println!("Starting chat session... (type /exit or /quit to leave)");
    println!("Available commands: /help, /commit, /push, /publish, /exit, /quit");
    println!();
    
    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;
    
    let mut conversation: Vec<ChatMessage> = Vec::new();
    
    loop {
        print!("You: ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Handle slash commands
        if input.starts_with('/') {
            match input {
                "/exit" | "/quit" => {
                    println!("Goodbye!");
                    break;
                }
                "/help" => {
                    show_chat_help();
                    continue;
                }
                "/commit" => {
                    if let Err(e) = handle_commit(false).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/commit all" => {
                    if let Err(e) = handle_commit(true).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/push" => {
                    if let Err(e) = handle_push(false).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/push force" => {
                    if let Err(e) = handle_push(true).await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                "/publish" => {
                    if let Err(e) = handle_publish().await {
                        println!("Error: {}", e);
                    }
                    continue;
                }
                _ => {
                    println!("Unknown command: {}. Type /help for available commands.", input);
                    continue;
                }
            }
        }
        
        // Add user message to conversation
        conversation.push(ChatMessage::user(input));
        
        // Get AI response
        match client.chat(&conversation).await {
            Ok(response) => {
                println!("AI: {}", response);
                // Add AI response to conversation
                conversation.push(ChatMessage::assistant(response));
            }
            Err(e) => {
                println!("Error getting AI response: {}", e);
            }
        }
    }
    
    Ok(())
}

pub fn show_chat_help() {
    println!("Chat Commands:");
    println!("  /help          Show this help message");
    println!("  /commit        Commit changes with AI-generated message");
    println!("  /commit all    Stage all changes and commit with AI-generated message");
    println!("  /push          Push changes to remote repository");
    println!("  /push force    Force push changes to remote repository");
    println!("  /publish       Publish project to appropriate registry");
    println!("  /exit, /quit   Exit the chat session");
}