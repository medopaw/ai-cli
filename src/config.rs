use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Include generated default values from build.rs
include!(concat!(env!("OUT_DIR"), "/default_config.rs"));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AiConfig,
    pub git: GitConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiConfig {
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

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig {
                provider: DEFAULT_AI_PROVIDER.to_string(),
                model: DEFAULT_AI_MODEL.to_string(),
                base_url: DEFAULT_AI_BASE_URL.to_string(),
            },
            git: GitConfig {
                commit_prompt: DEFAULT_GIT_COMMIT_PROMPT.to_string(),
            },
            history: HistoryConfig { 
                enabled: DEFAULT_HISTORY_ENABLED 
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Create config file with default values
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
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

    pub fn config_path() -> Result<PathBuf> {
        let home = home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".ai.conf.toml"))
    }

    pub fn history_db_path() -> Result<PathBuf> {
        let home = home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".ai.history.db"))
    }
}