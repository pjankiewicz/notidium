//! Text embedder using fastembed

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;

use crate::error::{Error, Result};

/// Text embedder wrapper with separate models for prose and code
pub struct Embedder {
    prose_model: Mutex<TextEmbedding>,
    code_model: Mutex<TextEmbedding>,
}

impl Embedder {
    /// Create a new embedder with default models
    /// - Prose: BGE-small-en-v1.5 (384 dimensions)
    /// - Code: Jina-embeddings-v2-base-code (768 dimensions)
    pub fn new() -> Result<Self> {
        let prose_options = InitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_show_download_progress(true);
        let prose_model = TextEmbedding::try_new(prose_options)
            .map_err(|e| Error::Embedding(format!("Failed to load prose model: {}", e)))?;

        let code_options = InitOptions::new(EmbeddingModel::JinaEmbeddingsV2BaseCode)
            .with_show_download_progress(true);
        let code_model = TextEmbedding::try_new(code_options)
            .map_err(|e| Error::Embedding(format!("Failed to load code model: {}", e)))?;

        Ok(Self {
            prose_model: Mutex::new(prose_model),
            code_model: Mutex::new(code_model),
        })
    }

    /// Embed a single text using the prose model
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_prose(text).await
    }

    /// Embed a single text using the prose model
    pub async fn embed_prose(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        let model = self.prose_model.lock().unwrap();

        let embeddings = model
            .embed(vec![text], None)
            .map_err(|e| Error::Embedding(e.to_string()))?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Embedding("No embedding generated".into()))
    }

    /// Embed a single text using the code model
    pub async fn embed_code(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        let model = self.code_model.lock().unwrap();

        let embeddings = model
            .embed(vec![text], None)
            .map_err(|e| Error::Embedding(e.to_string()))?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Embedding("No embedding generated".into()))
    }

    /// Embed a batch of texts using the prose model
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_batch_prose(texts).await
    }

    /// Embed a batch of texts using the prose model
    pub async fn embed_batch_prose(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let model = self.prose_model.lock().unwrap();

        model
            .embed(texts, None)
            .map_err(|e| Error::Embedding(e.to_string()))
    }

    /// Embed a batch of texts using the code model
    pub async fn embed_batch_code(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let model = self.code_model.lock().unwrap();

        model
            .embed(texts, None)
            .map_err(|e| Error::Embedding(e.to_string()))
    }

    /// Get prose embedding dimension
    pub fn prose_dimension(&self) -> usize {
        384 // BGE-small-en-v1.5 dimension
    }

    /// Get code embedding dimension
    pub fn code_dimension(&self) -> usize {
        768 // Jina-embeddings-v2-base-code dimension
    }

    /// Get embedding dimension (prose, for backwards compatibility)
    pub fn dimension(&self) -> usize {
        self.prose_dimension()
    }
}

impl Default for Embedder {
    fn default() -> Self {
        Self::new().expect("Failed to create embedder")
    }
}
