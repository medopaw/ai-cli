use crate::config::{AiConfig, GitConfig};
use anyhow::{anyhow, Result};
use ai::clients::ollama;
use ai::chat_completions::{ChatCompletion, ChatCompletionMessage, ChatCompletionRequestBuilder};

pub struct AiClient {
    config: AiConfig,
    git_config: GitConfig,
    ollama_client: ollama::Client,
}

impl AiClient {
    pub fn new(config: AiConfig, git_config: GitConfig) -> Result<Self> {
        let ollama_client = ollama::Client::from_url(&config.base_url)
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;
        
        Ok(Self { 
            config, 
            git_config,
            ollama_client,
        })
    }

    pub async fn ask(&self, question: &str) -> Result<String> {
        let request = ChatCompletionRequestBuilder::default()
            .model(&self.config.model)
            .messages(vec![
                ChatCompletionMessage::User(question.into()),
            ])
            .build()
            .map_err(|e| anyhow!("Failed to build chat request: {}", e))?;

        let response = self.ollama_client.chat_completions(&request).await
            .map_err(|e| anyhow!("Failed to get AI response: {}", e))?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            Err(anyhow!("No response from AI"))
        }
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let ai_messages: Vec<ChatCompletionMessage> = messages.iter()
            .map(|msg| {
                match msg.role.as_str() {
                    "user" => ChatCompletionMessage::User(msg.content.clone().into()),
                    "assistant" => ChatCompletionMessage::Assistant(msg.content.clone().into()),
                    _ => ChatCompletionMessage::User(msg.content.clone().into()), // Default to user
                }
            })
            .collect();

        let request = ChatCompletionRequestBuilder::default()
            .model(&self.config.model)
            .messages(ai_messages)
            .build()
            .map_err(|e| anyhow!("Failed to build chat request: {}", e))?;

        let response = self.ollama_client.chat_completions(&request).await
            .map_err(|e| anyhow!("Failed to get AI response: {}", e))?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            Err(anyhow!("No response from AI"))
        }
    }

    pub async fn generate_commit_message(&self, diff: &str) -> Result<String> {
        let prompt = self.git_config.commit_prompt.replace("{diff}", diff);
        self.ask(&prompt).await
    }

    pub async fn is_available(&self) -> bool {
        // Simple health check by trying to make a minimal request
        let request = ChatCompletionRequestBuilder::default()
            .model(&self.config.model)
            .messages(vec![ChatCompletionMessage::User("hello".into())])
            .build();
        
        if let Ok(req) = request {
            self.ollama_client.chat_completions(&req).await.is_ok()
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}