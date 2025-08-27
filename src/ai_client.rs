use crate::config::{ProviderConfig, CommandAiConfig, GitConfig};
use anyhow::{anyhow, Result};
use ai::clients::ollama;
use ai::chat_completions::{ChatCompletion, ChatCompletionMessage, ChatCompletionRequestBuilder};

pub struct AiClient {
    provider_config: ProviderConfig,
    command_config: CommandAiConfig,
    git_config: GitConfig,
    ollama_client: ollama::Client,
}

impl AiClient {
    pub fn new(provider_config: ProviderConfig, command_config: CommandAiConfig, git_config: GitConfig) -> Result<Self> {
        let ollama_client = ollama::Client::from_url(&provider_config.base_url)
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;
        
        Ok(Self { 
            provider_config, 
            command_config,
            git_config,
            ollama_client,
        })
    }

    pub async fn ask(&self, question: &str) -> Result<String> {
        let request = ChatCompletionRequestBuilder::default()
            .model(&self.command_config.model)
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
            .model(&self.command_config.model)
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

    pub async fn analyze_and_fix_error(&self, history_context: &str, user_prompt: &str) -> Result<String> {
        let base_prompt = r#"You are an expert system administrator and developer that helps fix command line errors.

Analyze the provided context to identify the root cause of errors and provide solutions.

IMPORTANT: Pay attention to the "Analysis Type" field:
- If it's "Shell startup error", focus on configuration file issues (.zshrc, .bashrc, .gitconfig, etc.)
- If it's command execution error, focus on the failed command in the history

Please follow this format:
## Analysis
[Brief explanation of what went wrong]

## Root Cause  
[The specific reason for the failure]

## Solution
[Step by step explanation of how to fix it]

## Commands
```bash
# Commands to fix the issue (one per line)
command1
command2
command3
```

The commands section should only contain the actual shell commands that need to be executed, one per line, without explanations or comments inside the code block.

For shell startup errors, common causes include:
- Corrupted config files (.zshrc, .bashrc, .gitconfig)
- Lock files that weren't cleaned up properly
- Permission issues with config files
- Path issues or missing dependencies
- Syntax errors in shell configuration

"#;

        let full_prompt = if user_prompt.is_empty() {
            format!("{}\n\nTerminal History and Context:\n{}", base_prompt, history_context)
        } else {
            format!("{}\n\nAdditional Context from User: {}\n\nTerminal History and Context:\n{}", 
                   base_prompt, user_prompt, history_context)
        };

        self.ask(&full_prompt).await
    }

    pub async fn is_available(&self) -> bool {
        // Simple health check by trying to make a minimal request
        let request = ChatCompletionRequestBuilder::default()
            .model(&self.command_config.model)
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