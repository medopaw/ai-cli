use anyhow::Result;
use crate::config::Config;
use crate::ai_client::AiClient;

pub async fn handle_ask(question: &str) -> Result<()> {
    println!("Loading configuration...");
    let config = Config::load()?;
    let client = AiClient::new(config.ai, config.git)?;
    
    println!("Asking AI: {}", question);
    let response = client.ask(question).await?;
    println!("{}", response);
    
    Ok(())
}