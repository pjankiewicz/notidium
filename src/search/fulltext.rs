//! Tantivy full-text search index

use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::error::Result;
use crate::types::{Note, SearchResult};

/// Full-text search index using Tantivy
pub struct FullTextIndex {
    index: Index,
    reader: IndexReader,
    writer: std::sync::Mutex<IndexWriter>,

    // Schema fields
    id_field: Field,
    title_field: Field,
    content_field: Field,
    tags_field: Field,
}

impl FullTextIndex {
    /// Create or open an index at the given path
    pub fn open(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path)?;

        let mut schema_builder = Schema::builder();
        // ID field must be STRING (indexed but not tokenized) to support delete_term
        let id_field = schema_builder.add_text_field("id", tantivy::schema::STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED); // Also store content for snippets
        let tags_field = schema_builder.add_text_field("tags", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path)?
        } else {
            Index::create_in_dir(path, schema.clone())?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let writer = index.writer(50_000_000)?; // 50MB heap

        Ok(Self {
            index,
            reader,
            writer: std::sync::Mutex::new(writer),
            id_field,
            title_field,
            content_field,
            tags_field,
        })
    }

    /// Index a note
    pub fn index_note(&self, note: &Note) -> Result<()> {
        let writer = self.writer.lock().unwrap();

        // Delete existing document with same ID
        let id_term = tantivy::Term::from_field_text(self.id_field, &note.id.to_string());
        writer.delete_term(id_term);

        // Add new document
        let tags = note.tags().join(" ");
        writer.add_document(doc!(
            self.id_field => note.id.to_string(),
            self.title_field => note.title.clone(),
            self.content_field => note.content.clone(),
            self.tags_field => tags,
        ))?;

        Ok(())
    }

    /// Commit pending changes
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        // Force reader to reload so changes are immediately visible
        self.reader.reload()?;
        Ok(())
    }

    /// Search notes
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.title_field, self.content_field, self.tags_field],
        );

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let parsed_query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let id = doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = doc
                .get_first(self.title_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Generate snippet from content
            let snippet = generate_snippet(&content, &query_terms, 200);

            results.push(SearchResult {
                note_id: id,
                title,
                snippet,
                score,
                chunk_type: None,
                tags: Vec::new(), // Will be enriched by handler if needed
                updated_at: None, // Will be enriched by handler if needed
            });
        }

        Ok(results)
    }

    /// Delete a note from the index
    pub fn delete_note(&self, note_id: &str) -> Result<()> {
        let writer = self.writer.lock().unwrap();
        let term = tantivy::Term::from_field_text(self.id_field, note_id);
        writer.delete_term(term);
        Ok(())
    }

    /// Rebuild the entire index from notes
    pub fn rebuild(&self, notes: &[Note]) -> Result<()> {
        {
            let writer = self.writer.lock().unwrap();
            writer.delete_all_documents()?;
        }

        for note in notes {
            self.index_note(note)?;
        }

        self.commit()?;
        Ok(())
    }
}

/// Generate a snippet from content, trying to center around query terms
fn generate_snippet(content: &str, query_terms: &[&str], max_len: usize) -> String {
    if content.is_empty() {
        return String::new();
    }

    let content_lower = content.to_lowercase();

    // Try to find the first occurrence of any query term
    let mut best_pos: Option<usize> = None;
    for term in query_terms {
        if let Some(pos) = content_lower.find(term) {
            match best_pos {
                None => best_pos = Some(pos),
                Some(existing) if pos < existing => best_pos = Some(pos),
                _ => {}
            }
        }
    }

    // Calculate snippet bounds
    let (start, end) = match best_pos {
        Some(pos) => {
            // Center the snippet around the match
            let half_len = max_len / 2;
            let start = pos.saturating_sub(half_len);
            let end = (pos + half_len).min(content.len());
            (start, end)
        }
        None => {
            // No match found, just take from the beginning
            (0, max_len.min(content.len()))
        }
    };

    // Adjust to word boundaries
    let adjusted_start = if start > 0 {
        content[..start]
            .rfind(char::is_whitespace)
            .map(|p| p + 1)
            .unwrap_or(start)
    } else {
        0
    };

    let adjusted_end = if end < content.len() {
        content[end..]
            .find(char::is_whitespace)
            .map(|p| end + p)
            .unwrap_or(end)
    } else {
        content.len()
    };

    // Build snippet
    let mut snippet = String::new();
    if adjusted_start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(content[adjusted_start..adjusted_end].trim());
    if adjusted_end < content.len() {
        snippet.push_str("...");
    }

    // Clean up newlines
    snippet.replace('\n', " ").replace("  ", " ")
}
