use anyhow::Result;
use crate::config::Config;
use crate::ai_client::AiClient;

pub async fn handle_ask(question: &str) -> Result<()> {
    println!("Loading configuration...");
    let config = Config::load()?;
    let (provider_config, command_config) = config.get_conversation_ai_config()?;
    let client = AiClient::new(provider_config.clone(), command_config.clone(), config.git)?;
    
    println!("Asking AI: {}", question);
    let response = client.ask(question).await?;
    println!("{}", response);
    
    Ok(())
}