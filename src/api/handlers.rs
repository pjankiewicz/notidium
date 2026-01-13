//! API request handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::routes::AppState;
use crate::types::{ChunkType, Note, NoteMeta, SearchResult};

// Query parameters

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListParams {
    /// Maximum number of results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of results to skip
    #[serde(default)]
    pub offset: usize,
    /// Filter by tag name
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
    /// Search query string
    pub q: String,
    /// Maximum number of results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
}

// Request bodies

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNoteRequest {
    /// Title of the note
    pub title: String,
    /// Markdown content of the note
    pub content: String,
    /// Optional tags to assign
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateNoteRequest {
    /// Updated title (optional)
    pub title: Option<String>,
    /// Updated markdown content (optional)
    pub content: Option<String>,
    /// Updated tags (optional)
    pub tags: Option<Vec<String>>,
    /// Pin status (optional)
    pub is_pinned: Option<bool>,
    /// Archive status (optional)
    pub is_archived: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CaptureRequest {
    /// Content to capture
    pub content: String,
    /// Optional source identifier
    pub source: Option<String>,
}

// Response types

#[derive(Debug, Serialize, ToSchema)]
pub struct NoteResponse {
    /// Unique note identifier
    pub id: String,
    /// Note title
    pub title: String,
    /// URL-friendly slug
    pub slug: String,
    /// Full markdown content
    pub content: String,
    /// Associated tags
    pub tags: Vec<String>,
    /// ISO 8601 creation timestamp
    pub created_at: String,
    /// ISO 8601 last update timestamp
    pub updated_at: String,
    /// Whether note is pinned
    pub is_pinned: bool,
    /// Whether note is archived
    pub is_archived: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse {
    /// List of note metadata
    pub notes: Vec<NoteMeta>,
    /// Total count of matching notes
    pub total: usize,
    /// Current offset
    pub offset: usize,
    /// Page size limit
    pub limit: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// Search results with scores
    pub results: Vec<SearchResult>,
    /// Total number of results
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TagsResponse {
    /// List of all tags
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    /// Total number of notes
    pub note_count: usize,
    /// Total number of indexed chunks
    pub chunk_count: usize,
    /// Total number of unique tags
    pub tag_count: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// API version
    pub version: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

// Default values

fn default_limit() -> usize {
    50
}

// Helper function to chunk and embed a note
async fn index_note_chunks(state: &AppState, note: &Note) {
    // Create chunks from the note
    let chunks = state.chunker.chunk_note(note);

    if chunks.is_empty() {
        return;
    }

    // Embed each chunk
    for mut chunk in chunks {
        // Always embed with prose model
        match state.embedder.embed_prose(&chunk.content).await {
            Ok(embedding) => {
                chunk.prose_embedding = Some(embedding);
                chunk.embedded_at = Some(chrono::Utc::now());
            }
            Err(e) => {
                tracing::warn!("Failed to embed chunk: {}", e);
                continue;
            }
        }

        // For code blocks, also embed with code model
        if matches!(chunk.chunk_type, ChunkType::CodeBlock { .. }) {
            match state.embedder.embed_code(&chunk.content).await {
                Ok(embedding) => {
                    chunk.code_embedding = Some(embedding);
                }
                Err(e) => {
                    tracing::warn!("Failed to embed code chunk: {}", e);
                }
            }
        }

        // Add to semantic search
        let mut semantic = state.semantic.write().await;
        semantic.add_chunk(chunk);
    }

    tracing::debug!("Indexed chunks for note {}", note.id);
}

// Helper function to remove chunks for a note
async fn remove_note_chunks(state: &AppState, note_id: uuid::Uuid) {
    let mut semantic = state.semantic.write().await;
    semantic.remove_chunks_for_note(note_id);
    tracing::debug!("Removed chunks for note {}", note_id);
}

// Handlers

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// List all notes with pagination
#[utoipa::path(
    get,
    path = "/api/notes",
    params(ListParams),
    responses(
        (status = 200, description = "List of notes", body = ListResponse)
    ),
    tag = "notes"
)]
pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<ListResponse> {
    let notes = state
        .store
        .list_paginated(params.offset, params.limit, params.tag.as_deref())
        .await;

    let all_notes = state.store.list().await;
    let total = all_notes.iter().filter(|n| !n.is_deleted && !n.is_archived).count();

    Json(ListResponse {
        notes: notes.iter().map(NoteMeta::from).collect(),
        total,
        offset: params.offset,
        limit: params.limit,
    })
}

/// Get a single note by ID
#[utoipa::path(
    get,
    path = "/api/notes/{id}",
    params(
        ("id" = String, Path, description = "Note UUID")
    ),
    responses(
        (status = 200, description = "Note found", body = NoteResponse),
        (status = 400, description = "Invalid note ID", body = ErrorResponse),
        (status = 404, description = "Note not found", body = ErrorResponse)
    ),
    tag = "notes"
)]
pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid note ID".into(),
            }),
        )
    })?;

    let note = state.store.get(uuid).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Note not found".into(),
            }),
        )
    })?;

    let tags = note.tags();
    Ok(Json(NoteResponse {
        id: note.id.to_string(),
        title: note.title,
        slug: note.slug,
        content: note.content,
        tags,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
        is_pinned: note.is_pinned,
        is_archived: note.is_archived,
    }))
}

/// Create a new note
#[utoipa::path(
    post,
    path = "/api/notes",
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "Note created", body = NoteResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "notes"
)]
pub async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<NoteResponse>), (StatusCode, Json<ErrorResponse>)> {
    let note = state
        .store
        .create(req.title, req.content, req.tags)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    // Index the note for fulltext search
    if let Err(e) = state.fulltext.index_note(&note) {
        tracing::warn!("Failed to index note: {}", e);
    }
    let _ = state.fulltext.commit();

    // Index chunks for semantic search
    index_note_chunks(&state, &note).await;

    let tags = note.tags();
    Ok((
        StatusCode::CREATED,
        Json(NoteResponse {
            id: note.id.to_string(),
            title: note.title,
            slug: note.slug,
            content: note.content,
            tags,
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
            is_pinned: note.is_pinned,
            is_archived: note.is_archived,
        }),
    ))
}

/// Update an existing note
#[utoipa::path(
    put,
    path = "/api/notes/{id}",
    params(
        ("id" = String, Path, description = "Note UUID")
    ),
    request_body = UpdateNoteRequest,
    responses(
        (status = 200, description = "Note updated", body = NoteResponse),
        (status = 400, description = "Invalid note ID", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "notes"
)]
pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid note ID".into(),
            }),
        )
    })?;

    let note = state
        .store
        .update_full(uuid, req.title, req.content, req.tags, req.is_pinned, req.is_archived)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    // Re-index for fulltext search
    if let Err(e) = state.fulltext.index_note(&note) {
        tracing::warn!("Failed to re-index note: {}", e);
    }
    let _ = state.fulltext.commit();

    // Re-index chunks for semantic search (remove old, add new)
    remove_note_chunks(&state, uuid).await;
    index_note_chunks(&state, &note).await;

    let tags = note.tags();
    Ok(Json(NoteResponse {
        id: note.id.to_string(),
        title: note.title,
        slug: note.slug,
        content: note.content,
        tags,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
        is_pinned: note.is_pinned,
        is_archived: note.is_archived,
    }))
}

/// Delete a note (soft delete)
#[utoipa::path(
    delete,
    path = "/api/notes/{id}",
    params(
        ("id" = String, Path, description = "Note UUID")
    ),
    responses(
        (status = 204, description = "Note deleted"),
        (status = 400, description = "Invalid note ID", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "notes"
)]
pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid note ID".into(),
            }),
        )
    })?;

    state.store.delete(uuid).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Remove from fulltext index
    let _ = state.fulltext.delete_note(&id);
    let _ = state.fulltext.commit();

    // Remove chunks from semantic search
    remove_note_chunks(&state, uuid).await;

    Ok(StatusCode::NO_CONTENT)
}

/// Full-text search across notes
#[utoipa::path(
    get,
    path = "/api/search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse)
    ),
    tag = "search"
)]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Json<SearchResponse> {
    let results = state
        .fulltext
        .search(&params.q, params.limit)
        .unwrap_or_default();

    // Enrich with note metadata
    let mut enriched = Vec::new();
    for mut result in results {
        if let Ok(uuid) = result.note_id.parse::<uuid::Uuid>() {
            if let Some(note) = state.store.get(uuid).await {
                result.tags = note.tags();
                result.updated_at = Some(note.updated_at.to_rfc3339());
                enriched.push(result);
            }
        }
    }

    let total = enriched.len();
    Json(SearchResponse { results: enriched, total })
}

/// Semantic search using embeddings
#[utoipa::path(
    get,
    path = "/api/search/semantic",
    params(SearchParams),
    responses(
        (status = 200, description = "Semantic search results", body = SearchResponse)
    ),
    tag = "search"
)]
pub async fn semantic_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Json<SearchResponse> {
    let semantic = state.semantic.read().await;
    let results = semantic
        .search(&params.q, params.limit)
        .await
        .unwrap_or_default();

    // Enrich with note metadata and filter out results where note doesn't exist
    let mut enriched = Vec::new();
    for mut result in results {
        if let Ok(uuid) = result.note_id.parse::<uuid::Uuid>() {
            if let Some(note) = state.store.get(uuid).await {
                result.title = note.title.clone();
                result.tags = note.tags();
                result.updated_at = Some(note.updated_at.to_rfc3339());
                enriched.push(result);
            } else {
                // Skip results where the note no longer exists
                tracing::debug!("Skipping search result for missing note: {}", result.note_id);
            }
        }
    }

    let total = enriched.len();
    Json(SearchResponse {
        results: enriched,
        total,
    })
}

/// Find notes related to a given note
#[utoipa::path(
    get,
    path = "/api/notes/{id}/related",
    params(
        ("id" = String, Path, description = "Note UUID"),
        ListParams
    ),
    responses(
        (status = 200, description = "Related notes", body = SearchResponse),
        (status = 400, description = "Invalid note ID", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "search"
)]
pub async fn find_related(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid note ID".into(),
            }),
        )
    })?;

    let semantic = state.semantic.read().await;
    let results = semantic
        .find_similar(uuid, params.limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let total = results.len();
    Ok(Json(SearchResponse { results, total }))
}

/// Quick capture content as a new note
#[utoipa::path(
    post,
    path = "/api/capture",
    request_body = CaptureRequest,
    responses(
        (status = 201, description = "Capture created", body = NoteResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "notes"
)]
pub async fn quick_capture(
    State(state): State<AppState>,
    Json(req): Json<CaptureRequest>,
) -> Result<(StatusCode, Json<NoteResponse>), (StatusCode, Json<ErrorResponse>)> {
    let note = state
        .store
        .quick_capture(req.content, req.source)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    // Index for fulltext search
    if let Err(e) = state.fulltext.index_note(&note) {
        tracing::warn!("Failed to index capture: {}", e);
    }
    let _ = state.fulltext.commit();

    // Index chunks for semantic search
    index_note_chunks(&state, &note).await;

    let tags = note.tags();
    Ok((
        StatusCode::CREATED,
        Json(NoteResponse {
            id: note.id.to_string(),
            title: note.title,
            slug: note.slug,
            content: note.content,
            tags,
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
            is_pinned: note.is_pinned,
            is_archived: note.is_archived,
        }),
    ))
}

/// List all unique tags
#[utoipa::path(
    get,
    path = "/api/tags",
    responses(
        (status = 200, description = "List of tags", body = TagsResponse)
    ),
    tag = "metadata"
)]
pub async fn list_tags(State(state): State<AppState>) -> Json<TagsResponse> {
    let notes = state.store.list().await;
    let mut tags = std::collections::HashSet::new();

    for note in &notes {
        for tag in note.tags() {
            tags.insert(tag);
        }
    }

    let mut sorted: Vec<_> = tags.into_iter().collect();
    sorted.sort();

    Json(TagsResponse { tags: sorted })
}

/// Get vault statistics
#[utoipa::path(
    get,
    path = "/api/stats",
    responses(
        (status = 200, description = "Vault statistics", body = StatsResponse)
    ),
    tag = "metadata"
)]
pub async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let notes = state.store.list().await;
    let note_count = notes.iter().filter(|n| !n.is_deleted).count();

    let semantic = state.semantic.read().await;
    let chunk_count = semantic.chunk_count();

    let mut tags = std::collections::HashSet::new();
    for note in &notes {
        for tag in note.tags() {
            tags.insert(tag.to_lowercase());
        }
    }

    Json(StatsResponse {
        note_count,
        chunk_count,
        tag_count: tags.len(),
    })
}
