//! Semantic search using embeddings

use std::sync::Arc;

use crate::embed::Embedder;
use crate::error::Result;
use crate::types::{Chunk, QueryType, SearchResult};

/// Semantic search engine
pub struct SemanticSearch {
    embedder: Arc<Embedder>,
    chunks: Vec<Chunk>,
}

impl SemanticSearch {
    pub fn new(embedder: Arc<Embedder>) -> Self {
        Self {
            embedder,
            chunks: Vec::new(),
        }
    }

    /// Load chunks with embeddings
    pub fn load_chunks(&mut self, chunks: Vec<Chunk>) {
        self.chunks = chunks;
    }

    /// Add a chunk
    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.chunks.push(chunk);
    }

    /// Remove all chunks for a given note
    pub fn remove_chunks_for_note(&mut self, note_id: uuid::Uuid) {
        self.chunks.retain(|c| c.note_id != note_id);
    }

    /// Search using semantic similarity
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        if self.chunks.is_empty() {
            return Ok(Vec::new());
        }

        let query_type = QueryType::classify(query);

        // Embed query and score chunks based on query type:
        // - Prose/Hybrid: use prose_embedding (all chunks have this)
        // - Code: use code_embedding (only code chunks have this, for specialized matching)
        let mut scored: Vec<(f32, &Chunk)> = match query_type {
            QueryType::Prose | QueryType::Hybrid => {
                // Use prose model - finds all content including code via natural language
                let query_embedding = self.embedder.embed_prose(query).await?;
                self.chunks
                    .iter()
                    .filter_map(|chunk| {
                        chunk.prose_embedding.as_ref().map(|emb| {
                            (cosine_similarity(&query_embedding, emb), chunk)
                        })
                    })
                    .collect()
            }
            QueryType::Code => {
                // Use code model - specialized for code syntax queries
                let query_embedding = self.embedder.embed_code(query).await?;
                self.chunks
                    .iter()
                    .filter_map(|chunk| {
                        chunk.code_embedding.as_ref().map(|emb| {
                            (cosine_similarity(&query_embedding, emb), chunk)
                        })
                    })
                    .collect()
            }
        };

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results, deduplicating by note_id
        let mut results = Vec::new();
        let mut seen_notes = std::collections::HashSet::new();

        for (score, chunk) in scored {
            if seen_notes.contains(&chunk.note_id) {
                continue;
            }
            seen_notes.insert(chunk.note_id);

            // Create snippet from chunk content
            let snippet = chunk
                .content
                .chars()
                .take(200)
                .collect::<String>()
                .replace('\n', " ");

            results.push(SearchResult {
                note_id: chunk.note_id.to_string(),
                title: String::new(), // Will be filled in by caller
                snippet,
                score,
                chunk_type: Some(format!("{:?}", chunk.chunk_type)),
                tags: Vec::new(), // Will be filled in by caller
                updated_at: None, // Will be filled in by caller
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    /// Find similar notes to a given note
    pub async fn find_similar(&self, note_id: uuid::Uuid, limit: usize) -> Result<Vec<SearchResult>> {
        // Get chunks for this note
        let note_chunks: Vec<&Chunk> = self
            .chunks
            .iter()
            .filter(|c| c.note_id == note_id)
            .collect();

        if note_chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Average the embeddings of this note's chunks
        let embeddings: Vec<&Vec<f32>> = note_chunks
            .iter()
            .filter_map(|c| c.prose_embedding.as_ref())
            .collect();

        if embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let dim = embeddings[0].len();
        let mut avg_embedding = vec![0.0f32; dim];
        for emb in &embeddings {
            for (i, &v) in emb.iter().enumerate() {
                avg_embedding[i] += v;
            }
        }
        for v in &mut avg_embedding {
            *v /= embeddings.len() as f32;
        }

        // Score all other notes' chunks
        let mut scored: Vec<(f32, &Chunk)> = self
            .chunks
            .iter()
            .filter(|c| c.note_id != note_id)
            .filter_map(|chunk| {
                chunk.prose_embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(&avg_embedding, emb);
                    (score, chunk)
                })
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate by note_id
        let mut results = Vec::new();
        let mut seen_notes = std::collections::HashSet::new();

        for (score, chunk) in scored {
            if seen_notes.contains(&chunk.note_id) {
                continue;
            }
            seen_notes.insert(chunk.note_id);

            let snippet = chunk
                .content
                .chars()
                .take(200)
                .collect::<String>()
                .replace('\n', " ");

            results.push(SearchResult {
                note_id: chunk.note_id.to_string(),
                title: String::new(),
                snippet,
                score,
                chunk_type: Some(format!("{:?}", chunk.chunk_type)),
                tags: Vec::new(),
                updated_at: None,
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Clear all chunks
    pub fn clear(&mut self) {
        self.chunks.clear();
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}
