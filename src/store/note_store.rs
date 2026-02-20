//! File-based note storage with manifest-based ID tracking

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::types::{Frontmatter, Note};
use super::manifest::Manifest;

/// File-based note storage with in-memory cache and manifest-based ID tracking
pub struct NoteStore {
    config: Config,
    notes: Arc<RwLock<HashMap<uuid::Uuid, Note>>>,
    manifest: Arc<RwLock<Manifest>>,
}

impl NoteStore {
    pub fn new(config: Config) -> Self {
        // Load or create manifest
        let manifest_path = config.data_dir().join("manifest.json");
        let manifest = Manifest::load(&manifest_path).unwrap_or_default();

        Self {
            config,
            notes: Arc::new(RwLock::new(HashMap::new())),
            manifest: Arc::new(RwLock::new(manifest)),
        }
    }

    /// Get the manifest path
    fn manifest_path(&self) -> PathBuf {
        self.config.data_dir().join("manifest.json")
    }

    /// Save the manifest to disk
    async fn save_manifest(&self) -> Result<()> {
        let manifest = self.manifest.read().await;
        manifest.save(&self.manifest_path())
    }

    /// Load all notes from disk
    pub async fn load_all(&self) -> Result<Vec<Note>> {
        let notes_path = self.config.notes_path();
        let mut notes = Vec::new();

        if !notes_path.exists() {
            return Ok(notes);
        }

        self.load_directory(&notes_path, &mut notes).await?;

        // Update cache and prune deleted notes from manifest
        let mut cache = self.notes.write().await;
        let existing_paths: Vec<PathBuf> = notes.iter().map(|n| n.file_path.clone()).collect();

        {
            let mut manifest = self.manifest.write().await;
            let _deleted_ids = manifest.prune_deleted(&existing_paths);
            // Could notify search index about deleted notes here
        }

        for note in &notes {
            cache.insert(note.id, note.clone());
        }

        // Save manifest after loading
        self.save_manifest().await?;

        Ok(notes)
    }

    fn load_directory<'a>(
        &'a self,
        dir: &'a Path,
        notes: &'a mut Vec<Note>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let entries = std::fs::read_dir(dir)?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Skip hidden directories
                    if path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with('.'))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    self.load_directory(&path, notes).await?;
                } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    match self.load_note_from_file(&path).await {
                        Ok(note) => notes.push(note),
                        Err(e) => {
                            tracing::warn!("Failed to load note {:?}: {}", path, e);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Load a single note from a file
    pub async fn load_note_from_file(&self, path: &Path) -> Result<Note> {
        let content = tokio::fs::read_to_string(path).await?;
        let relative_path = path
            .strip_prefix(&self.config.notes_path())
            .unwrap_or(path)
            .to_path_buf();

        let (frontmatter, body) = parse_frontmatter(&content);

        let title = frontmatter
            .as_ref()
            .and_then(|fm| fm.custom.get("title"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| extract_title_from_content(&body))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        let content_hash = compute_hash(&content);

        // Get or create stable ID and retrieve persisted timestamps from manifest
        let (id, persisted_created_at, persisted_updated_at) = {
            let mut manifest = self.manifest.write().await;
            let id = manifest.get_or_create_id(&relative_path, &content_hash);
            let entry = manifest.get_entry(&relative_path);
            let created_at = entry.and_then(|e| e.created_at);
            let updated_at = entry.and_then(|e| e.updated_at);
            (id, created_at, updated_at)
        };

        let mut note = Note::new(title, content.clone(), relative_path);
        note.id = id;
        note.content_hash = content_hash;
        note.frontmatter = frontmatter;

        // Restore timestamps from manifest, falling back to file modification time
        let file_mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(chrono::DateTime::<chrono::Utc>::from);

        note.created_at = persisted_created_at
            .or(file_mtime)
            .unwrap_or(note.created_at);
        note.updated_at = persisted_updated_at
            .or(file_mtime)
            .unwrap_or(note.updated_at);

        // Backfill timestamps into manifest if they were missing (migration)
        if persisted_created_at.is_none() || persisted_updated_at.is_none() {
            let mut manifest = self.manifest.write().await;
            if persisted_created_at.is_none() {
                if let Some(entry) = manifest.get_entry_mut(&note.file_path) {
                    entry.created_at = Some(note.created_at);
                }
            }
            if persisted_updated_at.is_none() {
                if let Some(entry) = manifest.get_entry_mut(&note.file_path) {
                    entry.updated_at = Some(note.updated_at);
                }
            }
        }

        Ok(note)
    }

    /// Get a note by ID
    pub async fn get(&self, id: uuid::Uuid) -> Option<Note> {
        let cache = self.notes.read().await;
        cache.get(&id).cloned()
    }

    /// Get a note by title (fuzzy match)
    pub async fn get_by_title(&self, title: &str) -> Option<Note> {
        let cache = self.notes.read().await;
        let title_lower = title.to_lowercase();

        // Exact match first
        if let Some(note) = cache.values().find(|n| n.title.to_lowercase() == title_lower) {
            return Some(note.clone());
        }

        // Fuzzy match
        cache
            .values()
            .find(|n| n.title.to_lowercase().contains(&title_lower))
            .cloned()
    }

    /// Get all notes
    pub async fn list(&self) -> Vec<Note> {
        let cache = self.notes.read().await;
        cache.values().cloned().collect()
    }

    /// Get notes with pagination
    pub async fn list_paginated(
        &self,
        offset: usize,
        limit: usize,
        tag: Option<&str>,
    ) -> Vec<Note> {
        let cache = self.notes.read().await;
        let mut notes: Vec<_> = cache
            .values()
            .filter(|n| !n.is_deleted && !n.is_archived)
            .filter(|n| {
                if let Some(tag) = tag {
                    n.tags().iter().any(|t| t.to_lowercase() == tag.to_lowercase())
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by updated_at descending
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        notes.into_iter().skip(offset).take(limit).collect()
    }

    /// Create a new note
    pub async fn create(&self, title: String, content: String, tags: Option<Vec<String>>) -> Result<Note> {
        let slug = slug::slugify(&title);
        let filename = format!("{}.md", slug);
        let file_path = PathBuf::from(&filename);
        let full_path = self.config.notes_path().join(&file_path);

        if full_path.exists() {
            return Err(Error::NoteAlreadyExists(title));
        }

        // Build frontmatter if tags provided
        let mut note_content = String::new();
        if let Some(ref tags) = tags {
            if !tags.is_empty() {
                note_content.push_str("---\n");
                note_content.push_str(&format!("tags: [{}]\n", tags.join(", ")));
                note_content.push_str("---\n\n");
            }
        }
        note_content.push_str(&content);

        let content_hash = compute_hash(&note_content);

        // Get ID from manifest
        let note_id = {
            let mut manifest = self.manifest.write().await;
            manifest.get_or_create_id(&file_path, &content_hash)
        };

        // Write to disk
        tokio::fs::write(&full_path, &note_content).await?;

        // Create note object with the stable ID
        let mut note = Note::new(title, note_content, file_path);
        note.id = note_id;
        note.content_hash = content_hash;
        if let Some(tags) = tags {
            note.frontmatter = Some(Frontmatter {
                tags,
                ..Default::default()
            });
        }

        // Update cache
        let mut cache = self.notes.write().await;
        cache.insert(note.id, note.clone());

        // Save manifest
        self.save_manifest().await?;

        Ok(note)
    }

    /// Update a note's content
    pub async fn update(&self, id: uuid::Uuid, content: String) -> Result<Note> {
        let mut cache = self.notes.write().await;

        let note = cache
            .get_mut(&id)
            .ok_or_else(|| Error::NoteNotFound(id.to_string()))?;

        note.content = content.clone();
        note.updated_at = chrono::Utc::now();
        note.content_hash = compute_hash(&content);

        // Update manifest hash and timestamps
        {
            let mut manifest = self.manifest.write().await;
            manifest.update_hash(&note.file_path, &note.content_hash);
            manifest.update_timestamps(&note.file_path, note.updated_at);
        }

        // Write to disk
        let full_path = self.config.notes_path().join(&note.file_path);
        tokio::fs::write(&full_path, &content).await?;

        let result = note.clone();
        drop(cache);

        self.save_manifest().await?;

        Ok(result)
    }

    /// Update a note with all fields
    pub async fn update_full(
        &self,
        id: uuid::Uuid,
        title: Option<String>,
        content: Option<String>,
        tags: Option<Vec<String>>,
        is_pinned: Option<bool>,
        is_archived: Option<bool>,
    ) -> Result<Note> {
        let mut cache = self.notes.write().await;

        let note = cache
            .get_mut(&id)
            .ok_or_else(|| Error::NoteNotFound(id.to_string()))?;

        // Update fields if provided
        if let Some(new_title) = title {
            note.title = new_title;
        }
        if let Some(pinned) = is_pinned {
            note.is_pinned = pinned;
        }
        if let Some(archived) = is_archived {
            note.is_archived = archived;
        }

        // Handle tags update
        if let Some(new_tags) = tags {
            if let Some(ref mut fm) = note.frontmatter {
                fm.tags = new_tags;
            } else {
                note.frontmatter = Some(Frontmatter {
                    tags: new_tags,
                    ..Default::default()
                });
            }
        }

        // Handle content update and rebuild the full file content
        // Always strip frontmatter from content - tags come from separate field
        let body_content = if let Some(new_content) = content {
            // Strip frontmatter from incoming content to avoid duplicates
            let (_, body) = parse_frontmatter(&new_content);
            body
        } else {
            // Extract body from existing content
            let (_, body) = parse_frontmatter(&note.content);
            body
        };

        // Rebuild content with frontmatter
        let mut new_file_content = String::new();
        if let Some(ref fm) = note.frontmatter {
            if !fm.tags.is_empty() || !fm.custom.is_empty() {
                new_file_content.push_str("---\n");
                if !fm.tags.is_empty() {
                    new_file_content.push_str(&format!("tags: [{}]\n", fm.tags.join(", ")));
                }
                for (key, value) in &fm.custom {
                    if key != "tags" {
                        // Serialize the YAML value back to string
                        if let Ok(yaml_str) = serde_yaml::to_string(value) {
                            let yaml_str = yaml_str.trim();
                            new_file_content.push_str(&format!("{}: {}\n", key, yaml_str));
                        }
                    }
                }
                new_file_content.push_str("---\n\n");
            }
        }
        new_file_content.push_str(&body_content);

        note.content = new_file_content.clone();
        note.updated_at = chrono::Utc::now();
        note.content_hash = compute_hash(&new_file_content);

        // Update manifest hash and timestamps
        {
            let mut manifest = self.manifest.write().await;
            manifest.update_hash(&note.file_path, &note.content_hash);
            manifest.update_timestamps(&note.file_path, note.updated_at);
        }

        // Write to disk
        let full_path = self.config.notes_path().join(&note.file_path);
        tokio::fs::write(&full_path, &new_file_content).await?;

        let result = note.clone();
        drop(cache);

        self.save_manifest().await?;

        Ok(result)
    }

    /// Append content to a note
    pub async fn append(&self, id: uuid::Uuid, content: String) -> Result<Note> {
        let note = self
            .get(id)
            .await
            .ok_or_else(|| Error::NoteNotFound(id.to_string()))?;

        let new_content = format!("{}\n\n{}", note.content, content);
        self.update(id, new_content).await
    }

    /// Delete a note (soft delete)
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let mut cache = self.notes.write().await;

        let note = cache
            .get_mut(&id)
            .ok_or_else(|| Error::NoteNotFound(id.to_string()))?;

        note.is_deleted = true;
        note.deleted_at = Some(chrono::Utc::now());

        // Move to trash folder
        let full_path = self.config.notes_path().join(&note.file_path);
        let trash_path = self.config.data_dir().join("trash").join(&note.file_path);

        if let Some(parent) = trash_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::rename(&full_path, &trash_path).await?;

        Ok(())
    }

    /// Quick capture to inbox
    pub async fn quick_capture(&self, content: String, source: Option<String>) -> Result<Note> {
        let now = chrono::Utc::now();
        let title = format!("Capture {}", now.format("%Y-%m-%d %H:%M"));

        let mut note_content = String::new();
        note_content.push_str("---\n");
        note_content.push_str(&format!("captured_at: {}\n", now.to_rfc3339()));
        if let Some(source) = &source {
            note_content.push_str(&format!("source: \"{}\"\n", source));
        }
        note_content.push_str("---\n\n");
        note_content.push_str(&content);

        let slug = slug::slugify(&title);
        let filename = format!("{}.md", slug);
        let file_path = PathBuf::from("inbox").join(&filename);
        let full_path = self.config.notes_path().join(&file_path);

        // Ensure inbox exists
        tokio::fs::create_dir_all(full_path.parent().unwrap()).await?;

        let content_hash = compute_hash(&note_content);

        // Get ID from manifest
        let note_id = {
            let mut manifest = self.manifest.write().await;
            manifest.get_or_create_id(&file_path, &content_hash)
        };

        // Write to disk
        tokio::fs::write(&full_path, &note_content).await?;

        // Create note object with the stable ID
        let mut note = Note::new(title, note_content, file_path);
        note.id = note_id;
        note.content_hash = content_hash;

        // Update cache
        let mut cache = self.notes.write().await;
        cache.insert(note.id, note.clone());

        // Save manifest
        self.save_manifest().await?;

        Ok(note)
    }

    /// Check which notes need re-indexing
    pub async fn get_notes_needing_reindex(&self) -> Vec<Note> {
        let cache = self.notes.read().await;
        let manifest = self.manifest.read().await;

        cache
            .values()
            .filter(|note| manifest.needs_reindex(&note.file_path, &note.content_hash))
            .cloned()
            .collect()
    }

    /// Mark a note as indexed
    pub async fn mark_indexed(&self, id: uuid::Uuid) -> Result<()> {
        let cache = self.notes.read().await;
        if let Some(note) = cache.get(&id) {
            let mut manifest = self.manifest.write().await;
            manifest.mark_indexed(&note.file_path);
            drop(manifest);
            drop(cache);
            self.save_manifest().await?;
        }
        Ok(())
    }

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Parse frontmatter from markdown content
fn parse_frontmatter(content: &str) -> (Option<Frontmatter>, String) {
    if !content.starts_with("---") {
        return (None, content.to_string());
    }

    let rest = &content[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let yaml = &rest[..end_idx];
        let body = &rest[end_idx + 4..].trim_start();

        match serde_yaml::from_str::<Frontmatter>(yaml) {
            Ok(fm) => (Some(fm), body.to_string()),
            Err(_) => (None, content.to_string()),
        }
    } else {
        (None, content.to_string())
    }
}

/// Extract title from first heading or first line
fn extract_title_from_content(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].to_string());
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            // Use first non-empty, non-heading line as title
            let title = trimmed.chars().take(100).collect::<String>();
            return Some(title);
        }
    }
    None
}

fn compute_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}
