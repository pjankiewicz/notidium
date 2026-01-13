//! Core types for Notidium

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

/// A note in the knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub file_path: PathBuf,
    pub content_hash: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,

    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,

    pub frontmatter: Option<Frontmatter>,
}

impl Note {
    pub fn new(title: String, content: String, file_path: PathBuf) -> Self {
        let now = Utc::now();
        let slug = slug::slugify(&title);
        let content_hash = compute_hash(&content);

        Self {
            id: Uuid::new_v4(),
            title,
            slug,
            content,
            file_path,
            content_hash,
            created_at: now,
            updated_at: now,
            accessed_at: now,
            is_pinned: false,
            is_archived: false,
            is_deleted: false,
            deleted_at: None,
            frontmatter: None,
        }
    }

    /// Extract tags from frontmatter and inline #tags
    pub fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();

        // From frontmatter
        if let Some(fm) = &self.frontmatter {
            tags.extend(fm.tags.clone());
        }

        // TODO: Extract inline #tags from content

        tags
    }
}

/// YAML frontmatter metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_yaml::Value>,
}

/// A chunk of content for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: Uuid,
    pub note_id: Uuid,
    pub content: String,
    pub chunk_type: ChunkType,
    pub language: Option<String>,

    pub start_line: u32,
    pub end_line: u32,
    pub start_offset: u32,
    pub end_offset: u32,

    pub prose_embedding: Option<Vec<f32>>,
    pub code_embedding: Option<Vec<f32>>,

    pub embedding_model: Option<String>,
    pub embedded_at: Option<DateTime<Utc>>,
}

impl Chunk {
    pub fn new(note_id: Uuid, content: String, chunk_type: ChunkType) -> Self {
        Self {
            id: Uuid::new_v4(),
            note_id,
            content,
            chunk_type,
            language: None,
            start_line: 0,
            end_line: 0,
            start_offset: 0,
            end_offset: 0,
            prose_embedding: None,
            code_embedding: None,
            embedding_model: None,
            embedded_at: None,
        }
    }

    pub fn is_code(&self) -> bool {
        matches!(self.chunk_type, ChunkType::CodeBlock { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ChunkType {
    Prose,
    Heading { level: u8 },
    CodeBlock { language: String, title: Option<String> },
    ListItem,
    Blockquote,
}

/// A tag in the knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub parent: Option<String>,
    pub source: TagSource,
    pub note_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TagSource {
    Manual,
    AutoLanguage,
    AutoFramework,
    AutoConcept,
}

/// A link between notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: Uuid,
    pub source_note_id: Uuid,
    pub target_note_id: Option<Uuid>,
    pub target_raw: String,
    pub link_type: LinkType,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkType {
    WikiLink,
    HeadingLink,
    BlockReference,
    ExternalUrl,
}

/// Search result with score
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SearchResult {
    pub note_id: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_type: Option<String>,
    /// Tags from the note
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// ISO 8601 last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Query type classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryType {
    Prose,
    Code,
    Hybrid,
}

impl QueryType {
    /// Classify a query as prose-like, code-like, or hybrid
    pub fn classify(query: &str) -> Self {
        let code_signals = [
            query.contains("::"),
            query.contains("->"),
            query.contains("()"),
            query.contains("{}"),
            query.contains("[]"),
            query.contains(".unwrap"),
            query.contains("async "),
            query.contains("fn "),
            query.contains("def "),
            query.contains("func "),
            query.contains("const "),
            query.contains("let "),
            query.contains("var "),
            query.contains(".rs"),
            query.contains(".py"),
            query.contains(".ts"),
            query.contains(".js"),
            has_camel_case(query),
            has_snake_case(query),
        ];

        let code_score = code_signals.iter().filter(|&&x| x).count();

        if code_score >= 2 {
            QueryType::Code
        } else if code_score == 1 {
            QueryType::Hybrid
        } else {
            QueryType::Prose
        }
    }
}

/// Note metadata for listing (without full content)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub is_pinned: bool,
    pub is_archived: bool,
}

impl From<&Note> for NoteMeta {
    fn from(note: &Note) -> Self {
        Self {
            id: note.id.to_string(),
            title: note.title.clone(),
            slug: note.slug.clone(),
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
            tags: note.tags(),
            is_pinned: note.is_pinned,
            is_archived: note.is_archived,
        }
    }
}

// Helper functions

fn compute_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn has_camel_case(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    for i in 1..chars.len() {
        if chars[i].is_uppercase() && chars[i - 1].is_lowercase() {
            return true;
        }
    }
    false
}

fn has_snake_case(s: &str) -> bool {
    s.contains('_')
        && s.chars()
            .any(|c| c.is_alphabetic() && c.is_lowercase())
}
