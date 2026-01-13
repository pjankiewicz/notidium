//! Configuration for Notidium

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Root directory for notes (default: ~/Notidium)
    pub vault_path: PathBuf,

    /// Subdirectory for notes within vault
    #[serde(default = "default_notes_dir")]
    pub notes_dir: String,

    /// Subdirectory for attachments
    #[serde(default = "default_attachments_dir")]
    pub attachments_dir: String,

    /// Subdirectory for templates
    #[serde(default = "default_templates_dir")]
    pub templates_dir: String,

    /// HTTP server port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// MCP server port (for HTTP transport)
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,

    /// Embedding settings
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// Search settings
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model for prose embeddings
    #[serde(default = "default_prose_model")]
    pub prose_model: String,

    /// Batch size for embedding
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default number of results
    #[serde(default = "default_search_limit")]
    pub default_limit: usize,

    /// Maximum number of results
    #[serde(default = "default_max_limit")]
    pub max_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            vault_path: home.join("Notidium"),
            notes_dir: default_notes_dir(),
            attachments_dir: default_attachments_dir(),
            templates_dir: default_templates_dir(),
            http_port: default_http_port(),
            mcp_port: default_mcp_port(),
            embedding: EmbeddingConfig::default(),
            search: SearchConfig::default(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            prose_model: default_prose_model(),
            batch_size: default_batch_size(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_search_limit(),
            max_limit: default_max_limit(),
        }
    }
}

impl Config {
    /// Load config from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Load config from a specific vault path
    pub fn load_from_vault(vault_path: PathBuf) -> Result<Self> {
        let config_path = vault_path.join(".notidium").join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&content)?;
            config.vault_path = vault_path;
            Ok(config)
        } else {
            let mut config = Config::default();
            config.vault_path = vault_path;
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = self.data_dir().join("config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap())?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the default config path
    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| Error::Config("Could not find home directory".into()))?;
        Ok(home.join("Notidium").join(".notidium").join("config.toml"))
    }

    /// Path to notes directory
    pub fn notes_path(&self) -> PathBuf {
        self.vault_path.join(&self.notes_dir)
    }

    /// Path to attachments directory
    pub fn attachments_path(&self) -> PathBuf {
        self.vault_path.join(&self.attachments_dir)
    }

    /// Path to templates directory
    pub fn templates_path(&self) -> PathBuf {
        self.vault_path.join(&self.templates_dir)
    }

    /// Path to .notidium data directory
    pub fn data_dir(&self) -> PathBuf {
        self.vault_path.join(".notidium")
    }

    /// Path to SQLite database
    pub fn db_path(&self) -> PathBuf {
        self.data_dir().join("index.db")
    }

    /// Path to vector store directory
    pub fn vectors_path(&self) -> PathBuf {
        self.data_dir().join("vectors")
    }

    /// Path to tantivy index directory
    pub fn tantivy_path(&self) -> PathBuf {
        self.data_dir().join("tantivy")
    }

    /// Path to cache directory
    pub fn cache_path(&self) -> PathBuf {
        self.data_dir().join("cache")
    }

    /// Path to logs directory
    pub fn logs_path(&self) -> PathBuf {
        self.data_dir().join("logs")
    }

    /// Initialize vault directories
    pub fn init_vault(&self) -> Result<()> {
        std::fs::create_dir_all(self.notes_path())?;
        std::fs::create_dir_all(self.notes_path().join("inbox"))?;
        std::fs::create_dir_all(self.attachments_path())?;
        std::fs::create_dir_all(self.templates_path())?;
        std::fs::create_dir_all(self.data_dir())?;
        std::fs::create_dir_all(self.vectors_path())?;
        std::fs::create_dir_all(self.tantivy_path())?;
        std::fs::create_dir_all(self.cache_path())?;
        std::fs::create_dir_all(self.logs_path())?;

        // Create .notidiumignore if it doesn't exist
        let ignore_path = self.vault_path.join(".notidiumignore");
        if !ignore_path.exists() {
            std::fs::write(
                &ignore_path,
                "# Files and directories to ignore during indexing\n.notidium/\n.git/\nnode_modules/\n",
            )?;
        }

        Ok(())
    }
}

// Default value functions

fn default_notes_dir() -> String {
    "notes".to_string()
}

fn default_attachments_dir() -> String {
    "attachments".to_string()
}

fn default_templates_dir() -> String {
    "templates".to_string()
}

fn default_http_port() -> u16 {
    3939
}

fn default_mcp_port() -> u16 {
    3940
}

fn default_prose_model() -> String {
    "BAAI/bge-small-en-v1.5".to_string()
}

fn default_batch_size() -> usize {
    32
}

fn default_search_limit() -> usize {
    10
}

fn default_max_limit() -> usize {
    100
}
