//! SQLite metadata database

use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::error::Result;
use crate::types::Note;

/// SQLite database for note metadata
pub struct MetadataDb {
    conn: Mutex<Connection>,
}

impl MetadataDb {
    /// Open or create the database
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                slug TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                accessed_at TEXT NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_notes_slug ON notes(slug);
            CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at);

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                parent TEXT,
                source TEXT NOT NULL,
                note_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS note_tags (
                note_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (note_id, tag_id),
                FOREIGN KEY (note_id) REFERENCES notes(id),
                FOREIGN KEY (tag_id) REFERENCES tags(id)
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                content TEXT NOT NULL,
                chunk_type TEXT NOT NULL,
                language TEXT,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                embedding_model TEXT,
                embedded_at TEXT,
                FOREIGN KEY (note_id) REFERENCES notes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_note ON chunks(note_id);

            CREATE TABLE IF NOT EXISTS links (
                id TEXT PRIMARY KEY,
                source_note_id TEXT NOT NULL,
                target_note_id TEXT,
                target_raw TEXT NOT NULL,
                link_type TEXT NOT NULL,
                position INTEGER NOT NULL,
                FOREIGN KEY (source_note_id) REFERENCES notes(id),
                FOREIGN KEY (target_note_id) REFERENCES notes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_note_id);
            CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_note_id);
            "#,
        )?;

        Ok(())
    }

    /// Insert or update a note
    pub fn upsert_note(&self, note: &Note) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT INTO notes (id, title, slug, file_path, content_hash, created_at, updated_at, accessed_at, is_pinned, is_archived, is_deleted, deleted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                slug = excluded.slug,
                file_path = excluded.file_path,
                content_hash = excluded.content_hash,
                updated_at = excluded.updated_at,
                accessed_at = excluded.accessed_at,
                is_pinned = excluded.is_pinned,
                is_archived = excluded.is_archived,
                is_deleted = excluded.is_deleted,
                deleted_at = excluded.deleted_at
            "#,
            params![
                note.id.to_string(),
                note.title,
                note.slug,
                note.file_path.to_string_lossy().to_string(),
                note.content_hash,
                note.created_at.to_rfc3339(),
                note.updated_at.to_rfc3339(),
                note.accessed_at.to_rfc3339(),
                note.is_pinned,
                note.is_archived,
                note.is_deleted,
                note.deleted_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        // Update tags
        if let Some(fm) = &note.frontmatter {
            for tag in &fm.tags {
                self.ensure_tag(tag)?;
                self.link_note_tag(&note.id.to_string(), tag)?;
            }
        }

        Ok(())
    }

    /// Ensure a tag exists
    fn ensure_tag(&self, tag: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tag_lower = tag.to_lowercase();

        conn.execute(
            r#"
            INSERT INTO tags (id, name, display_name, source, note_count)
            VALUES (?1, ?2, ?3, 'Manual', 0)
            ON CONFLICT(name) DO NOTHING
            "#,
            params![uuid::Uuid::new_v4().to_string(), tag_lower, tag],
        )?;

        Ok(())
    }

    /// Link a note to a tag
    fn link_note_tag(&self, note_id: &str, tag: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tag_lower = tag.to_lowercase();

        conn.execute(
            r#"
            INSERT INTO note_tags (note_id, tag_id)
            SELECT ?1, id FROM tags WHERE name = ?2
            ON CONFLICT DO NOTHING
            "#,
            params![note_id, tag_lower],
        )?;

        // Update tag count
        conn.execute(
            r#"
            UPDATE tags SET note_count = (
                SELECT COUNT(*) FROM note_tags WHERE tag_id = tags.id
            ) WHERE name = ?1
            "#,
            params![tag_lower],
        )?;

        Ok(())
    }

    /// Get all unique tags
    pub fn get_tags(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT display_name FROM tags ORDER BY name")?;

        let tags: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tags)
    }

    /// Delete a note
    pub fn delete_note(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM note_tags WHERE note_id = ?1", params![id])?;
        conn.execute("DELETE FROM chunks WHERE note_id = ?1", params![id])?;
        conn.execute("DELETE FROM links WHERE source_note_id = ?1", params![id])?;
        conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;

        Ok(())
    }
}
