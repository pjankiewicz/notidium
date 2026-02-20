//! Internal manifest for tracking note metadata
//!
//! Maps file paths to stable UUIDs and content hashes without polluting user files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Entry for a single note in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Stable UUID for this note
    pub id: Uuid,
    /// SHA-256 hash of file content (for change detection)
    pub content_hash: String,
    /// Last indexed timestamp
    pub indexed_at: Option<DateTime<Utc>>,
    /// When the note was first seen
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// When the note content was last modified
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Internal manifest tracking note paths to IDs and hashes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// Map from relative file path to entry
    entries: HashMap<PathBuf, ManifestEntry>,
}

impl Manifest {
    /// Load manifest from disk, or create empty if doesn't exist
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let manifest: Manifest = serde_json::from_str(&content)
                .map_err(|e| Error::Other(format!("Failed to parse manifest: {}", e)))?;
            Ok(manifest)
        } else {
            Ok(Self::default())
        }
    }

    /// Save manifest to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Other(format!("Failed to serialize manifest: {}", e)))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get or create an ID for a note path
    pub fn get_or_create_id(&mut self, path: &Path, content_hash: &str) -> Uuid {
        if let Some(entry) = self.entries.get(path) {
            entry.id
        } else {
            let id = Uuid::new_v4();
            let now = Utc::now();
            self.entries.insert(path.to_path_buf(), ManifestEntry {
                id,
                content_hash: content_hash.to_string(),
                indexed_at: None,
                created_at: Some(now),
                updated_at: Some(now),
            });
            id
        }
    }

    /// Get the entry for a note path
    pub fn get_entry(&self, path: &Path) -> Option<&ManifestEntry> {
        self.entries.get(path)
    }

    /// Get mutable entry for a note path
    pub fn get_entry_mut(&mut self, path: &Path) -> Option<&mut ManifestEntry> {
        self.entries.get_mut(path)
    }

    /// Update timestamps when note content changes
    pub fn update_timestamps(&mut self, path: &Path, updated_at: DateTime<Utc>) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.updated_at = Some(updated_at);
            // Ensure created_at is set (migration for old entries)
            if entry.created_at.is_none() {
                entry.created_at = Some(updated_at);
            }
        }
    }

    /// Get the ID for a note path (if exists)
    pub fn get_id(&self, path: &Path) -> Option<Uuid> {
        self.entries.get(path).map(|e| e.id)
    }

    /// Get entry by ID (reverse lookup)
    pub fn get_path_by_id(&self, id: Uuid) -> Option<&Path> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.id == id)
            .map(|(path, _)| path.as_path())
    }

    /// Update the content hash for a note
    pub fn update_hash(&mut self, path: &Path, content_hash: &str) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.content_hash = content_hash.to_string();
        }
    }

    /// Mark a note as indexed
    pub fn mark_indexed(&mut self, path: &Path) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.indexed_at = Some(Utc::now());
        }
    }

    /// Check if a note needs re-indexing (hash changed or never indexed)
    pub fn needs_reindex(&self, path: &Path, current_hash: &str) -> bool {
        match self.entries.get(path) {
            Some(entry) => {
                entry.indexed_at.is_none() || entry.content_hash != current_hash
            }
            None => true, // New note
        }
    }

    /// Get all notes that need re-indexing given current file states
    pub fn get_stale_notes<'a>(
        &self,
        current_notes: &'a [(PathBuf, String)], // (path, hash) pairs
    ) -> Vec<&'a PathBuf> {
        current_notes
            .iter()
            .filter(|(path, hash)| self.needs_reindex(path, hash))
            .map(|(path, _)| path)
            .collect()
    }

    /// Remove entries for notes that no longer exist
    pub fn prune_deleted(&mut self, existing_paths: &[PathBuf]) -> Vec<Uuid> {
        let existing_set: std::collections::HashSet<_> = existing_paths.iter().collect();
        let deleted: Vec<_> = self.entries
            .iter()
            .filter(|(path, _)| !existing_set.contains(path))
            .map(|(path, entry)| (path.clone(), entry.id))
            .collect();

        let deleted_ids: Vec<Uuid> = deleted.iter().map(|(_, id)| *id).collect();

        for (path, _) in deleted {
            self.entries.remove(&path);
        }

        deleted_ids
    }

    /// Get statistics
    pub fn stats(&self) -> ManifestStats {
        let total = self.entries.len();
        let indexed = self.entries.values().filter(|e| e.indexed_at.is_some()).count();
        ManifestStats { total, indexed }
    }
}

#[derive(Debug)]
pub struct ManifestStats {
    pub total: usize,
    pub indexed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_or_create_id_new() {
        let mut manifest = Manifest::default();
        let path = PathBuf::from("notes/test.md");
        let hash = "abc123";

        let id1 = manifest.get_or_create_id(&path, hash);
        let id2 = manifest.get_or_create_id(&path, hash);

        assert_eq!(id1, id2, "Same path should return same ID");
    }

    #[test]
    fn test_needs_reindex() {
        let mut manifest = Manifest::default();
        let path = PathBuf::from("test.md");

        // New note needs indexing
        assert!(manifest.needs_reindex(&path, "hash1"));

        // After adding, still needs indexing (not marked)
        manifest.get_or_create_id(&path, "hash1");
        assert!(manifest.needs_reindex(&path, "hash1"));

        // After marking indexed, doesn't need reindex
        manifest.mark_indexed(&path);
        assert!(!manifest.needs_reindex(&path, "hash1"));

        // After hash change, needs reindex again
        assert!(manifest.needs_reindex(&path, "hash2"));
    }

    #[test]
    fn test_prune_deleted() {
        let mut manifest = Manifest::default();

        let path1 = PathBuf::from("existing.md");
        let path2 = PathBuf::from("deleted.md");

        let _id1 = manifest.get_or_create_id(&path1, "h1");
        let id2 = manifest.get_or_create_id(&path2, "h2");

        let deleted = manifest.prune_deleted(&[path1.clone()]);

        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], id2);
        assert!(manifest.get_id(&path1).is_some());
        assert!(manifest.get_id(&path2).is_none());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.json");

        let mut manifest = Manifest::default();
        let path = PathBuf::from("test.md");
        let id = manifest.get_or_create_id(&path, "hash123");

        manifest.save(&manifest_path).unwrap();

        let loaded = Manifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.get_id(&path), Some(id));
    }
}
