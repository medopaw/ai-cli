use crate::config::{ProviderConfig, CommandAiConfig, GitConfig, Config};
use crate::git_ops::{DiffSegment, FileSummary, DiffStats};
use anyhow::{anyhow, Result};
use ai::clients::{ollama, openai};
use ai::chat_completions::{ChatCompletion, ChatCompletionMessage, ChatCompletionRequestBuilder};
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use std::sync::Arc;

pub enum AiClientType {
    Ollama(ollama::Client),
    OpenAi(openai::Client),
}

pub struct AiClient {
    provider_config: ProviderConfig,
    command_config: CommandAiConfig,
    git_config: GitConfig,
    client: AiClientType,
    full_config: Option<Config>,
}

impl AiClient {
    fn create_client(provider_name: &str, provider_config: &ProviderConfig) -> Result<AiClientType> {
        match provider_name {
            "ollama" => {
                let client = ollama::Client::from_url(&provider_config.base_url)
                    .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;
                Ok(AiClientType::Ollama(client))
            }
            "openai" | "deepseek" => {
                let client = if provider_config.api_key.is_empty() {
                    return Err(anyhow!("API key is required for {} provider", provider_name));
                } else {
                    openai::Client::from_url(&provider_config.api_key, &provider_config.base_url)
                        .map_err(|e| anyhow!("Failed to create OpenAI client: {}", e))?
                };
                Ok(AiClientType::OpenAi(client))
            }
            _ => Err(anyhow!("Unsupported provider: {}", provider_name)),
        }
    }
    
    #[allow(dead_code)]
    pub fn new(provider_config: ProviderConfig, command_config: CommandAiConfig, git_config: GitConfig) -> Result<Self> {
        let client = Self::create_client(&command_config.provider, &provider_config)?;
        
        Ok(Self { 
            provider_config, 
            command_config,
            git_config,
            client,
            full_config: None,
        })
    }

    pub fn new_with_full_config(provider_config: ProviderConfig, command_config: CommandAiConfig, git_config: GitConfig, full_config: Config) -> Result<Self> {
        let client = Self::create_client(&command_config.provider, &provider_config)?;
        
        Ok(Self { 
            provider_config, 
            command_config,
            git_config,
            client,
            full_config: Some(full_config),
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

        let response = match &self.client {
            AiClientType::Ollama(client) => {
                client.chat_completions(&request).await
                    .map_err(|e| self.handle_ollama_error(e))?
            }
            AiClientType::OpenAi(client) => {
                client.chat_completions(&request).await
                    .map_err(|e| anyhow!("OpenAI API error: {}", e))?
            }
        };

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

        let response = match &self.client {
            AiClientType::Ollama(client) => {
                client.chat_completions(&request).await
                    .map_err(|e| self.handle_ollama_error(e))?
            }
            AiClientType::OpenAi(client) => {
                client.chat_completions(&request).await
                    .map_err(|e| anyhow!("OpenAI API error: {}", e))?
            }
        };

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

    /// Summarize diff segments in parallel with controlled concurrency
    pub async fn summarize_diff_segments(&self, segments: Vec<DiffSegment>) -> Result<Vec<FileSummary>> {
        let max_concurrency = self.git_config.max_concurrency;
        let timeout_duration = Duration::from_secs(self.git_config.segment_timeout_seconds);
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        let total_segments = segments.len();
        println!("Analyzing large diff in {} segments...", total_segments);

        let mut tasks = Vec::new();
        for (index, segment) in segments.into_iter().enumerate() {
            let sem = semaphore.clone();
            let client_type = match &self.client {
                AiClientType::Ollama(client) => AiClientType::Ollama(client.clone()),
                AiClientType::OpenAi(client) => AiClientType::OpenAi(client.clone()),
            };
            let model = self.command_config.model.clone();
            
            let task = async move {
                let _permit = sem.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;
                
                println!("Processing segment {}/{}...", index + 1, total_segments);
                
                let result = timeout(timeout_duration, async {
                    Self::summarize_segment(&client_type, &model, &segment).await
                }).await;

                match result {
                    Ok(summary_result) => summary_result,
                    Err(_) => Err(anyhow!("Request timeout after {}s", timeout_duration.as_secs())),
                }
            };
            
            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut all_summaries = Vec::new();
        for task in tasks {
            let segment_summaries = task.await?;
            all_summaries.extend(segment_summaries);
        }

        println!("Analysis complete. Generating commit message...");
        Ok(all_summaries)
    }

    /// Summarize a single diff segment
    async fn summarize_segment(
        client: &AiClientType, 
        model: &str, 
        segment: &DiffSegment
    ) -> Result<Vec<FileSummary>> {
        let prompt = format!(
            "请简洁总结以下每个文件的变更(每个文件一行)：\n\n{}\n\n输出格式：\nfilename: 变更描述 (10字以内)\n\n示例：\nsrc/main.rs: 添加错误处理逻辑\nconfig.toml: 更新依赖版本",
            segment.content
        );

        let request = ChatCompletionRequestBuilder::default()
            .model(model)
            .messages(vec![ChatCompletionMessage::User(prompt.into())])
            .build()
            .map_err(|e| anyhow!("Failed to build chat request: {}", e))?;

        let response = match client {
            AiClientType::Ollama(ollama_client) => {
                ollama_client.chat_completions(&request).await
                    .map_err(|e| anyhow!("Ollama API error: {}", e))?
            }
            AiClientType::OpenAi(openai_client) => {
                openai_client.chat_completions(&request).await
                    .map_err(|e| anyhow!("OpenAI API error: {}", e))?
            }
        };

        let content = response.choices.first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| anyhow!("No response content from AI"))?;

        // Parse the response into FileSummary objects
        Self::parse_file_summaries(content, &segment.files)
    }

    /// Parse AI response into FileSummary objects
    fn parse_file_summaries(content: &str, expected_files: &[String]) -> Result<Vec<FileSummary>> {
        let mut summaries = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("输出格式") || line.starts_with("示例") {
                continue;
            }
            
            if let Some((filename, summary)) = line.split_once(':') {
                let filename = filename.trim().to_string();
                let summary = summary.trim().to_string();
                
                // Verify this file was actually in the segment
                if expected_files.iter().any(|f| f.contains(&filename) || filename.contains(f)) {
                    summaries.push(FileSummary { filename, summary });
                }
            }
        }
        
        // If parsing failed, create fallback summaries
        if summaries.is_empty() && !expected_files.is_empty() {
            for filename in expected_files {
                summaries.push(FileSummary {
                    filename: filename.clone(),
                    summary: "文件已修改".to_string(),
                });
            }
        }
        
        Ok(summaries)
    }

    /// Generate final commit message based on stats and file summaries
    pub async fn generate_final_commit_message(&self, stats: &DiffStats, file_summaries: &[FileSummary]) -> Result<String> {
        let stats_text = format!(
            "{} files changed, {} insertions(+), {} deletions(-)",
            stats.files_changed, stats.lines_added, stats.lines_deleted
        );

        let mut file_details = String::new();
        for summary in file_summaries.iter().take(10) { // Limit to prevent overflow
            file_details.push_str(&format!("- {}: {}\n", summary.filename, summary.summary));
        }
        
        if file_summaries.len() > 10 {
            file_details.push_str(&format!("- ... and {} more files\n", file_summaries.len() - 10));
        }

        let prompt = format!(
            "基于以下信息生成commit message：\n\n统计摘要：\n{}\n\n文件变更详情：\n{}\n\n生成符合conventional commits格式的一行commit message。\n描述必须以小写字母开头，不超过72字符。",
            stats_text, file_details
        );

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

    #[allow(dead_code)]
    pub async fn is_available(&self) -> bool {
        // Simple health check by trying to make a minimal request
        let request = ChatCompletionRequestBuilder::default()
            .model(&self.command_config.model)
            .messages(vec![ChatCompletionMessage::User("hello".into())])
            .build();
        
        if let Ok(req) = request {
            match &self.client {
                AiClientType::Ollama(client) => {
                    client.chat_completions(&req).await.is_ok()
                }
                AiClientType::OpenAi(client) => {
                    client.chat_completions(&req).await.is_ok()
                }
            }
        } else {
            false
        }
    }

    fn handle_ollama_error(&self, error: impl std::fmt::Display) -> anyhow::Error {
        let error_msg = error.to_string().to_lowercase();
        
        // Check for common model not found patterns
        if error_msg.contains("model") && (
            error_msg.contains("not found") ||
            error_msg.contains("not available") ||
            error_msg.contains("pull") ||
            error_msg.contains("does not exist") ||
            error_msg.contains("404")
        ) {
            let model_name = &self.command_config.model;
            {
                let command = format!("ollama pull {}", model_name);
                let total_width = std::cmp::max(41, command.len() + 4);
                let inner_width = total_width - 2; // 减去左右边框
                let padded_command = format!(" {:<width$}", command, width = inner_width - 1); // -1 因为前面有一个空格
                
                anyhow!(
                    "❌ Model '{}' is not installed.\n\n\
                     📥 To install this model, run:\n\
                     {}\n\
                     {}\n\
                     {}\n\n\
                     💡 After installation, run your ai command again.\n\
                     🔗 Available models: https://ollama.com/library",
                    model_name,
                    "─".repeat(total_width),
                    padded_command,
                    "─".repeat(total_width)
                )
            }
        } else if error_msg.contains("connection") || 
                  error_msg.contains("refused") || 
                  error_msg.contains("connect") ||
                  error_msg.contains("no such host") ||
                  error_msg.contains("network is unreachable") ||
                  error_msg.contains("connection reset") ||
                  (error_msg.trim() == "unknown error:" || error_msg.trim().is_empty()) {
            
            // Check if this might be because Ollama is not installed
            if error_msg.contains("connection refused") || 
               error_msg.contains("no such host") || 
               error_msg.trim() == "unknown error:" ||
               error_msg.trim().is_empty() {
                let models_info = self.get_all_configured_models();
                anyhow!(
                    "❌ Cannot connect to Ollama server.\n\n\
                     🔍 This might be because:\n\
                     1️⃣  Ollama is not installed\n\
                     2️⃣  Ollama service is not running\n\n\
                     📦 If Ollama is not installed, install it:\n\
                     ┌─────────────────────────────────────────┐\n\
                     │ brew install ollama                     │\n\
                     └─────────────────────────────────────────┘\n\
                     🌐 Or download from: https://ollama.com/download\n\n\
                     🚀 If Ollama is installed, start the service:\n\
                     ┌─────────────────────────────────────────┐\n\
                     │ ollama serve                            │\n\
                     └─────────────────────────────────────────┘\n\n\
                     📦 After Ollama is running, install your configured models:\n\
                     {}\n\
                     📍 Server URL: {}\n\
                     💡 Follow terminal instructions after installation",
                    models_info, self.provider_config.base_url
                )
            } else {
                anyhow!(
                    "❌ Cannot connect to Ollama server.\n\n\
                     🚀 Make sure Ollama is running:\n\
                     ┌─────────────────────────────────────────┐\n\
                     │ ollama serve                            │\n\
                     └─────────────────────────────────────────┘\n\n\
                     📍 Server URL: {}\n\
                     🔗 Install Ollama: https://ollama.com/download",
                    self.provider_config.base_url
                )
            }
        } else {
            anyhow!("Failed to get AI response: {}", error)
        }
    }

    fn get_all_configured_models(&self) -> String {
        if let Some(config) = &self.full_config {
            let mut models = std::collections::HashSet::new();
            
            // Collect all unique models from commands
            models.insert(&config.commands.git_operations.model);
            models.insert(&config.commands.conversation.model);
            models.insert(&config.commands.error_analysis.model);
            
            let mut models_vec: Vec<_> = models.into_iter().collect();
            models_vec.sort();
            
            let mut result = String::new();
            for (i, model) in models_vec.iter().enumerate() {
                let command = format!("ollama pull {}", model);
                let total_width = std::cmp::max(41, command.len() + 4);
                let inner_width = total_width - 2; // 减去左右边框
                let padded_command = format!(" {:<width$}", command, width = inner_width - 1); // -1 因为前面有一个空格
                
                result.push_str(&format!(
                    "     {}\n\
                     {}\n\
                     {}",
                    "─".repeat(total_width),
                    padded_command,
                    "─".repeat(total_width)
                ));
                if i < models_vec.len() - 1 {
                    result.push('\n');
                }
            }
            result
        } else {
            let command = format!("ollama pull {}", &self.command_config.model);
            let total_width = std::cmp::max(41, command.len() + 4);
            let inner_width = total_width - 2; // 减去左右边框
            let padded_command = format!(" {:<width$}", command, width = inner_width - 1); // -1 因为前面有一个空格
            
            format!(
                "     {}\n\
                 {}\n\
                 {}",
                "─".repeat(total_width),
                padded_command,
                "─".repeat(total_width)
            )
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