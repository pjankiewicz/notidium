//! MCP server implementation

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::embed::{Chunker, Embedder};
use crate::search::{FullTextIndex, SemanticSearch};
use crate::store::NoteStore;
use crate::types::{Note, NoteMeta, SearchResult};

/// MCP server for Notidium
#[derive(Clone)]
pub struct NotidiumServer {
    pub store: Arc<NoteStore>,
    pub fulltext: Arc<FullTextIndex>,
    pub semantic: Arc<RwLock<SemanticSearch>>,
    pub embedder: Arc<Embedder>,
    pub chunker: Arc<Chunker>,
    tool_router: ToolRouter<Self>,
}

// Tool parameter types

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchNotesParams {
    /// Search query
    pub query: String,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
    /// Use semantic search (default: true)
    pub semantic: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNoteParams {
    /// Note ID
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNoteByTitleParams {
    /// Note title (fuzzy match)
    pub title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListNotesParams {
    /// Maximum number of results (default: 50)
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindRelatedParams {
    /// Note ID to find related notes for
    pub note_id: String,
    /// Maximum number of results (default: 5)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchCodeParams {
    /// Code search query
    pub query: String,
    /// Filter by programming language
    pub language: Option<String>,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateNoteParams {
    /// Note title
    pub title: String,
    /// Note content (markdown)
    pub content: String,
    /// Tags for the note
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateNoteParams {
    /// Note ID
    pub id: String,
    /// New content
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppendToNoteParams {
    /// Note ID
    pub id: String,
    /// Content to append
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickCaptureParams {
    /// Content to capture
    pub content: String,
    /// Source context (URL, app name, etc.)
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteNoteParams {
    /// Note ID to delete
    pub id: String,
}

// Response types (serialized as strings for MCP)

#[derive(Debug, Serialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
    total: usize,
}

#[derive(Debug, Serialize)]
struct NoteResponse {
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    notes: Vec<NoteMeta>,
    total: usize,
    offset: usize,
    limit: usize,
}

// Server implementation

#[tool_router]
impl NotidiumServer {
    pub fn new(
        store: Arc<NoteStore>,
        fulltext: Arc<FullTextIndex>,
        semantic: Arc<RwLock<SemanticSearch>>,
        embedder: Arc<Embedder>,
        chunker: Arc<Chunker>,
    ) -> Self {
        Self {
            store,
            fulltext,
            semantic,
            embedder,
            chunker,
            tool_router: Self::tool_router(),
        }
    }

    /// Index a note: chunk it, embed chunks, and add to semantic search
    async fn index_note(&self, note: &Note) -> Result<usize, String> {
        // Remove old chunks for this note
        {
            let mut semantic = self.semantic.write().await;
            semantic.remove_chunks_for_note(note.id);
        }

        // Chunk the note
        let mut chunks = self.chunker.chunk_note(note);
        if chunks.is_empty() {
            return Ok(0);
        }

        // Separate code and prose chunks by index
        let mut code_indices: Vec<usize> = Vec::new();
        let mut prose_indices: Vec<usize> = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.is_code() {
                code_indices.push(i);
            } else {
                prose_indices.push(i);
            }
        }

        // Embed prose chunks with prose model
        if !prose_indices.is_empty() {
            let prose_texts: Vec<String> = prose_indices
                .iter()
                .map(|&i| chunks[i].content.clone())
                .collect();
            let prose_embeddings = self.embedder.embed_batch_prose(prose_texts).await.map_err(|e| e.to_string())?;

            for (idx, embedding) in prose_indices.iter().zip(prose_embeddings) {
                chunks[*idx].prose_embedding = Some(embedding);
                chunks[*idx].embedded_at = Some(chrono::Utc::now());
            }
        }

        // Embed code chunks with BOTH models:
        // - prose_embedding: so natural language queries can find code
        // - code_embedding: for specialized code-syntax queries
        if !code_indices.is_empty() {
            let code_texts: Vec<String> = code_indices
                .iter()
                .map(|&i| chunks[i].content.clone())
                .collect();

            // Generate both embeddings for code chunks
            let prose_embeddings = self.embedder.embed_batch_prose(code_texts.clone()).await.map_err(|e| e.to_string())?;
            let code_embeddings = self.embedder.embed_batch_code(code_texts).await.map_err(|e| e.to_string())?;

            for (idx, (prose_emb, code_emb)) in code_indices.iter().zip(prose_embeddings.into_iter().zip(code_embeddings)) {
                chunks[*idx].prose_embedding = Some(prose_emb);
                chunks[*idx].code_embedding = Some(code_emb);
                chunks[*idx].embedded_at = Some(chrono::Utc::now());
            }
        }

        // Add to semantic search
        let chunk_count = chunks.len();
        {
            let mut semantic = self.semantic.write().await;
            for chunk in chunks {
                semantic.add_chunk(chunk);
            }
        }

        // Index in fulltext as well
        if let Err(e) = self.fulltext.index_note(note) {
            tracing::warn!("Failed to index note in fulltext: {}", e);
        }
        let _ = self.fulltext.commit();

        Ok(chunk_count)
    }

    /// Search notes using full-text or semantic search
    #[tool(description = "Search notes in the knowledge base. Returns ranked results with snippets.")]
    async fn search_notes(&self, Parameters(params): Parameters<SearchNotesParams>) -> String {
        let limit = params.limit.unwrap_or(10);
        let use_semantic = params.semantic.unwrap_or(true);

        let results = if use_semantic {
            let semantic = self.semantic.read().await;
            match semantic.search(&params.query, limit).await {
                Ok(r) => r,
                Err(e) => return format!("Error: {}", e),
            }
        } else {
            match self.fulltext.search(&params.query, limit) {
                Ok(r) => r,
                Err(e) => return format!("Error: {}", e),
            }
        };

        // Enrich results with note titles
        let mut enriched = Vec::new();
        for mut result in results {
            if let Ok(uuid) = result.note_id.parse::<uuid::Uuid>() {
                if let Some(note) = self.store.get(uuid).await {
                    result.title = note.title;
                }
            }
            enriched.push(result);
        }

        let total = enriched.len();
        let response = SearchResponse {
            results: enriched,
            total,
        };

        serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
    }

    /// Get a note by its ID
    #[tool(description = "Get full note content by ID")]
    async fn get_note(&self, Parameters(params): Parameters<GetNoteParams>) -> String {
        let id = match params.id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(_) => return "Error: Invalid note ID".to_string(),
        };

        match self.store.get(id).await {
            Some(note) => {
                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            None => "Error: Note not found".to_string(),
        }
    }

    /// Get a note by title (fuzzy match)
    #[tool(description = "Get note by title with fuzzy matching")]
    async fn get_note_by_title(&self, Parameters(params): Parameters<GetNoteByTitleParams>) -> String {
        match self.store.get_by_title(&params.title).await {
            Some(note) => {
                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            None => "Error: Note not found".to_string(),
        }
    }

    /// List notes with pagination
    #[tool(description = "List notes with pagination and optional tag filter")]
    async fn list_notes(&self, Parameters(params): Parameters<ListNotesParams>) -> String {
        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        let notes = self
            .store
            .list_paginated(offset, limit, params.tag.as_deref())
            .await;

        let all_notes = self.store.list().await;
        let total = all_notes.iter().filter(|n| !n.is_deleted && !n.is_archived).count();

        let response = ListResponse {
            notes: notes.iter().map(NoteMeta::from).collect(),
            total,
            offset,
            limit,
        };

        serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
    }

    /// Find notes related to a given note
    #[tool(description = "Find semantically similar notes to a given note")]
    async fn find_related(&self, Parameters(params): Parameters<FindRelatedParams>) -> String {
        let note_id = match params.note_id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(_) => return "Error: Invalid note ID".to_string(),
        };

        let limit = params.limit.unwrap_or(5);
        let semantic = self.semantic.read().await;

        match semantic.find_similar(note_id, limit).await {
            Ok(results) => {
                // Enrich with titles
                let mut enriched = Vec::new();
                for mut result in results {
                    if let Ok(uuid) = result.note_id.parse::<uuid::Uuid>() {
                        if let Some(note) = self.store.get(uuid).await {
                            result.title = note.title;
                        }
                    }
                    enriched.push(result);
                }

                let total = enriched.len();
                let response = SearchResponse {
                    results: enriched,
                    total,
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Search code blocks specifically
    #[tool(description = "Search code blocks with optional language filter")]
    async fn search_code(&self, Parameters(params): Parameters<SearchCodeParams>) -> String {
        let limit = params.limit.unwrap_or(10);

        let semantic = self.semantic.read().await;
        let results = match semantic.search(&params.query, limit * 2).await {
            Ok(r) => r,
            Err(e) => return format!("Error: {}", e),
        };

        // Filter by language if specified
        let filtered: Vec<_> = if let Some(lang) = &params.language {
            results
                .into_iter()
                .filter(|r| {
                    r.chunk_type
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&lang.to_lowercase()))
                        .unwrap_or(false)
                })
                .take(limit)
                .collect()
        } else {
            results.into_iter().take(limit).collect()
        };

        let total = filtered.len();
        let response = SearchResponse {
            results: filtered,
            total,
        };

        serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
    }

    /// Create a new note
    #[tool(description = "Create a new note with optional tags")]
    async fn create_note(&self, Parameters(params): Parameters<CreateNoteParams>) -> String {
        match self.store.create(params.title, params.content, params.tags).await {
            Ok(note) => {
                // Index the note for search
                if let Err(e) = self.index_note(&note).await {
                    tracing::warn!("Failed to index note: {}", e);
                }

                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Update a note's content
    #[tool(description = "Replace note content")]
    async fn update_note(&self, Parameters(params): Parameters<UpdateNoteParams>) -> String {
        let id = match params.id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(_) => return "Error: Invalid note ID".to_string(),
        };

        match self.store.update(id, params.content).await {
            Ok(note) => {
                // Re-index the note
                if let Err(e) = self.index_note(&note).await {
                    tracing::warn!("Failed to re-index note: {}", e);
                }

                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Append content to a note
    #[tool(description = "Append content to an existing note")]
    async fn append_to_note(&self, Parameters(params): Parameters<AppendToNoteParams>) -> String {
        let id = match params.id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(_) => return "Error: Invalid note ID".to_string(),
        };

        match self.store.append(id, params.content).await {
            Ok(note) => {
                // Re-index the note
                if let Err(e) = self.index_note(&note).await {
                    tracing::warn!("Failed to re-index note: {}", e);
                }

                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Quick capture to inbox
    #[tool(description = "Quick capture content to inbox with optional source context")]
    async fn quick_capture(&self, Parameters(params): Parameters<QuickCaptureParams>) -> String {
        match self.store.quick_capture(params.content, params.source).await {
            Ok(note) => {
                // Index the captured note
                if let Err(e) = self.index_note(&note).await {
                    tracing::warn!("Failed to index captured note: {}", e);
                }

                let tags = note.tags();
                let response = NoteResponse {
                    id: note.id.to_string(),
                    title: note.title,
                    content: note.content,
                    tags,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.to_rfc3339(),
                };
                serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Delete a note by ID
    #[tool(description = "Delete a note by ID (moves to trash)")]
    async fn delete_note(&self, Parameters(params): Parameters<DeleteNoteParams>) -> String {
        let id = match params.id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(_) => return "Error: Invalid note ID".to_string(),
        };

        // Get note info before deletion for the response
        let note_title = self.store.get(id).await.map(|n| n.title.clone());

        // Remove from semantic search index
        {
            let mut semantic = self.semantic.write().await;
            semantic.remove_chunks_for_note(id);
        }

        // Remove from fulltext index
        if let Err(e) = self.fulltext.delete_note(&id.to_string()) {
            tracing::warn!("Failed to remove note from fulltext index: {}", e);
        }
        let _ = self.fulltext.commit();

        // Delete the note (moves to trash)
        match self.store.delete(id).await {
            Ok(()) => {
                let title = note_title.unwrap_or_else(|| id.to_string());
                format!("Successfully deleted note: {}", title)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Get knowledge base statistics
    #[tool(description = "Get statistics about the knowledge base")]
    async fn get_stats(&self) -> String {
        let notes = self.store.list().await;
        let note_count = notes.iter().filter(|n| !n.is_deleted).count();

        let semantic = self.semantic.read().await;
        let chunk_count = semantic.chunk_count();

        // Count unique tags
        let mut tags = std::collections::HashSet::new();
        for note in &notes {
            for tag in note.tags() {
                tags.insert(tag.to_lowercase());
            }
        }

        format!(
            "# Notidium Knowledge Base Stats\n\n\
            - **Notes:** {}\n\
            - **Chunks:** {}\n\
            - **Tags:** {}\n\
            - **Embedding Model:** BGE-small-en-v1.5 (384 dimensions)\n",
            note_count,
            chunk_count,
            tags.len()
        )
    }

    /// Get all tags
    #[tool(description = "Get all tags in the knowledge base")]
    async fn get_tags(&self) -> String {
        let notes = self.store.list().await;
        let mut tags = std::collections::HashSet::new();

        for note in &notes {
            for tag in note.tags() {
                tags.insert(tag);
            }
        }

        let mut sorted: Vec<_> = tags.into_iter().collect();
        sorted.sort();

        serde_json::to_string_pretty(&sorted).unwrap_or_else(|e| format!("Error: {}", e))
    }
}

#[tool_handler]
impl ServerHandler for NotidiumServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "notidium".into(),
                title: Some("Notidium Knowledge Base".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            instructions: Some("Notidium is a developer-focused knowledge base with semantic search. Use search_notes to find relevant content, get_note to retrieve full notes, and create_note or quick_capture to add new knowledge.".into()),
        }
    }
}

/// Run the MCP server on stdio
pub async fn serve_stdio(server: NotidiumServer) -> anyhow::Result<()> {
    tracing::info!("Starting MCP server on stdio...");
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Run the MCP server on HTTP
pub async fn serve_http(server: NotidiumServer, port: u16) -> anyhow::Result<()> {
    use axum::routing::get;
    use axum::Router;
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpService, StreamableHttpServerConfig,
    };
    use tokio_util::sync::CancellationToken;

    let store = server.store.clone();
    let fulltext = server.fulltext.clone();
    let semantic = server.semantic.clone();
    let embedder = server.embedder.clone();
    let chunker = server.chunker.clone();

    let ct = CancellationToken::new();

    let config = StreamableHttpServerConfig {
        cancellation_token: ct.clone(),
        ..Default::default()
    };

    let mcp_service = StreamableHttpService::new(
        move || Ok(NotidiumServer::new(store.clone(), fulltext.clone(), semantic.clone(), embedder.clone(), chunker.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest_service("/mcp", mcp_service);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Notidium MCP server running at http://{}/mcp", addr);
    tracing::info!("Health check available at http://{}/health", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
            ct.cancel();
        })
        .await?;

    Ok(())
}
