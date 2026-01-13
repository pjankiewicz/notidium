//! Error types for Notidium

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("Note already exists: {0}")]
    NoteAlreadyExists(String),

    #[error("Invalid note path: {0}")]
    InvalidNotePath(String),

    #[error("Invalid frontmatter: {0}")]
    InvalidFrontmatter(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("File watcher error: {0}")]
    Watcher(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<tantivy::TantivyError> for Error {
    fn from(err: tantivy::TantivyError) -> Self {
        Error::Search(err.to_string())
    }
}

impl From<tantivy::query::QueryParserError> for Error {
    fn from(err: tantivy::query::QueryParserError) -> Self {
        Error::Search(err.to_string())
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::Watcher(err.to_string())
    }
}
