//! Notidium - Developer-focused, local-first note-taking with semantic search and MCP integration

pub mod config;
pub mod error;
pub mod types;

pub mod store;
pub mod search;
pub mod embed;
pub mod mcp;
pub mod api;

pub use config::Config;
pub use error::{Error, Result};
pub use types::*;
