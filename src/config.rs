use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Include generated default values from build.rs
include!(concat!(env!("OUT_DIR"), "/default_config.rs"));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub providers: HashMap<String, ProviderConfig>,
    pub commands: CommandsConfig,
    pub git: GitConfig,
    pub history: HistoryConfig,
    // Keep old ai field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<LegacyAiConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandsConfig {
    pub git_operations: CommandAiConfig,
    pub conversation: CommandAiConfig,
    pub error_analysis: CommandAiConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandAiConfig {
    pub provider: String,
    pub model: String,
}

// Keep old structure for backward compatibility
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LegacyAiConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitConfig {
    pub commit_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryConfig {
    pub enabled: bool,
}

// For parsing legacy config files
#[derive(Debug, Serialize, Deserialize)]
struct LegacyConfigFormat {
    pub ai: LegacyAiConfig,
    pub git: GitConfig,
    pub history: HistoryConfig,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                api_key: "".to_string(),
                base_url: DEFAULT_OLLAMA_BASE_URL.to_string(),
            },
        );
        providers.insert(
            "deepseek".to_string(),
            ProviderConfig {
                api_key: "".to_string(), // Will be filled from user input
                base_url: DEFAULT_DEEPSEEK_BASE_URL.to_string(),
            },
        );

        Self {
            providers,
            commands: CommandsConfig {
                git_operations: CommandAiConfig {
                    provider: DEFAULT_AI_PROVIDER.to_string(),
                    model: DEFAULT_AI_MODEL.to_string(),
                },
                conversation: CommandAiConfig {
                    provider: DEFAULT_AI_PROVIDER.to_string(),
                    model: DEFAULT_AI_MODEL.to_string(),
                },
                error_analysis: CommandAiConfig {
                    provider: DEFAULT_AI_PROVIDER.to_string(),
                    model: DEFAULT_AI_MODEL.to_string(),
                },
            },
            git: GitConfig {
                commit_prompt: DEFAULT_GIT_COMMIT_PROMPT.to_string(),
            },
            history: HistoryConfig { 
                enabled: DEFAULT_HISTORY_ENABLED 
            },
            ai: None, // No legacy config by default
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Copy default config template and fill in missing values
            Self::create_default_config_file(&config_path)?;
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        
        // Try to parse as new format first
        match toml::from_str::<Config>(&content) {
            Ok(mut config) => {
                // Check if we need to migrate from legacy format
                if let Some(legacy_ai) = config.ai.clone() {
                    config = Self::migrate_from_legacy(config, legacy_ai)?;
                }
                Ok(config)
            },
            Err(_) => {
                // Try to parse as legacy format and migrate
                let legacy_config: LegacyConfigFormat = toml::from_str(&content)
                    .context("Failed to parse config file in both new and legacy formats")?;
                Self::migrate_legacy_config(legacy_config)
            }
        }
    }

    fn migrate_from_legacy(mut config: Config, legacy_ai: LegacyAiConfig) -> Result<Config> {
        // Update providers with legacy info if not already present
        if !config.providers.contains_key(&legacy_ai.provider) {
            config.providers.insert(
                legacy_ai.provider.clone(),
                ProviderConfig {
                    api_key: "".to_string(), // Will be filled from environment
                    base_url: legacy_ai.base_url.clone(),
                },
            );
        }

        // Update all commands to use legacy provider/model if they're still default
        let default_provider = DEFAULT_AI_PROVIDER;
        let default_model = DEFAULT_AI_MODEL;

        if config.commands.git_operations.provider == default_provider 
            && config.commands.git_operations.model == default_model {
            config.commands.git_operations.provider = legacy_ai.provider.clone();
            config.commands.git_operations.model = legacy_ai.model.clone();
        }

        if config.commands.conversation.provider == default_provider 
            && config.commands.conversation.model == default_model {
            config.commands.conversation.provider = legacy_ai.provider.clone();
            config.commands.conversation.model = legacy_ai.model.clone();
        }

        if config.commands.error_analysis.provider == default_provider 
            && config.commands.error_analysis.model == default_model {
            config.commands.error_analysis.provider = legacy_ai.provider.clone();
            config.commands.error_analysis.model = legacy_ai.model.clone();
        }

        // Clear legacy config after migration
        config.ai = None;

        // Save migrated config
        config.save()?;

        Ok(config)
    }

    fn migrate_legacy_config(legacy: LegacyConfigFormat) -> Result<Config> {
        let mut providers = HashMap::new();
        providers.insert(
            legacy.ai.provider.clone(),
            ProviderConfig {
                api_key: "".to_string(), // Will be filled from environment
                base_url: legacy.ai.base_url.clone(),
            },
        );

        let config = Config {
            providers,
            commands: CommandsConfig {
                git_operations: CommandAiConfig {
                    provider: legacy.ai.provider.clone(),
                    model: legacy.ai.model.clone(),
                },
                conversation: CommandAiConfig {
                    provider: legacy.ai.provider.clone(),
                    model: legacy.ai.model.clone(),
                },
                error_analysis: CommandAiConfig {
                    provider: legacy.ai.provider.clone(),
                    model: legacy.ai.model.clone(),
                },
            },
            git: legacy.git,
            history: legacy.history,
            ai: None,
        };

        // Save migrated config
        config.save()?;
        
        Ok(config)
    }

    pub fn get_ai_config_for_command(&self, command_type: &str) -> Result<(&ProviderConfig, &CommandAiConfig)> {
        let command_config = match command_type {
            "git_operations" => &self.commands.git_operations,
            "conversation" => &self.commands.conversation,
            "error_analysis" => &self.commands.error_analysis,
            _ => return Err(anyhow::anyhow!("Unknown command type: {}", command_type)),
        };

        let provider_config = self.providers.get(&command_config.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found in config", command_config.provider))?;

        Ok((provider_config, command_config))
    }

    pub fn get_git_operations_ai_config(&self) -> Result<(&ProviderConfig, &CommandAiConfig)> {
        self.get_ai_config_for_command("git_operations")
    }

    pub fn get_conversation_ai_config(&self) -> Result<(&ProviderConfig, &CommandAiConfig)> {
        self.get_ai_config_for_command("conversation")
    }

    pub fn get_error_analysis_ai_config(&self) -> Result<(&ProviderConfig, &CommandAiConfig)> {
        self.get_ai_config_for_command("error_analysis")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    fn create_default_config_file(config_path: &PathBuf) -> Result<()> {
        // Read the default config template (already includes all default values)
        let config_content = include_str!("../ai.conf.toml.default");
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Copy template directly to user config path
        fs::write(config_path, config_content)
            .context("Failed to copy default config template")?;
        
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".ai.conf.toml"))
    }

    #[allow(dead_code)]
    pub fn history_db_path() -> Result<PathBuf> {
        let home = home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".ai.history.db"))
    }
}
