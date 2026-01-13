//! API route definitions

use axum::{
    Router,
    routing::{get, post, put, delete},
    response::IntoResponse,
    http::{StatusCode, Uri, header},
};
use rust_embed::RustEmbed;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::handlers::{
    self, AttachmentResponse, CaptureRequest, CreateNoteRequest, ErrorResponse, HealthResponse,
    ListResponse, NoteResponse, SearchResponse, StatsResponse, TagsResponse, UpdateNoteRequest,
    UploadAttachmentRequest,
};
use crate::embed::{Chunker, Embedder};
use crate::mcp::NotidiumServer;
use crate::store::NoteStore;
use crate::search::{FullTextIndex, SemanticSearch};
use crate::types::{NoteMeta, SearchResult};

/// Embedded frontend assets (built from frontend/dist)
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Asset;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Notidium API",
        version = "0.1.0",
        description = "Developer-focused note-taking with semantic search and MCP integration"
    ),
    tags(
        (name = "notes", description = "Note management"),
        (name = "search", description = "Search operations"),
        (name = "metadata", description = "Tags and statistics"),
        (name = "attachments", description = "Attachment management"),
        (name = "health", description = "Health checks")
    ),
    paths(
        handlers::health,
        handlers::list_notes,
        handlers::get_note,
        handlers::create_note,
        handlers::update_note,
        handlers::delete_note,
        handlers::search,
        handlers::semantic_search,
        handlers::find_related,
        handlers::quick_capture,
        handlers::list_tags,
        handlers::get_stats,
        handlers::upload_attachment,
    ),
    components(schemas(
        NoteMeta,
        SearchResult,
        NoteResponse,
        ListResponse,
        SearchResponse,
        TagsResponse,
        StatsResponse,
        HealthResponse,
        ErrorResponse,
        CreateNoteRequest,
        UpdateNoteRequest,
        CaptureRequest,
        UploadAttachmentRequest,
        AttachmentResponse,
    ))
)]
pub struct ApiDoc;

/// Static file handler for embedded frontend assets
/// Serves files from the embedded `frontend/dist` directory with SPA routing support
async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Asset::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            match Asset::get("index.html") {
                Some(content) => {
                    let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                    ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
                }
                None => (StatusCode::NOT_FOUND, "Frontend not built. Run `make build` first.").into_response(),
            }
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<NoteStore>,
    pub fulltext: Arc<FullTextIndex>,
    pub semantic: Arc<tokio::sync::RwLock<SemanticSearch>>,
    pub embedder: Arc<Embedder>,
    pub chunker: Arc<Chunker>,
    pub attachments_path: std::path::PathBuf,
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let openapi = ApiDoc::openapi();

    Router::new()
        // Notes CRUD
        .route("/api/notes", get(handlers::list_notes))
        .route("/api/notes", post(handlers::create_note))
        .route("/api/notes/{id}", get(handlers::get_note))
        .route("/api/notes/{id}", put(handlers::update_note))
        .route("/api/notes/{id}", delete(handlers::delete_note))

        // Search
        .route("/api/search", get(handlers::search))
        .route("/api/search/semantic", get(handlers::semantic_search))
        .route("/api/notes/{id}/related", get(handlers::find_related))

        // Quick actions
        .route("/api/capture", post(handlers::quick_capture))

        // Attachments
        .route("/api/attachments", post(handlers::upload_attachment))
        .route("/api/attachments/{filename}", get(handlers::get_attachment))

        // Metadata
        .route("/api/tags", get(handlers::list_tags))
        .route("/api/stats", get(handlers::get_stats))

        // Health
        .route("/health", get(handlers::health))

        // OpenAPI spec and Swagger UI
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", openapi))

        // Static files (frontend)
        .fallback(static_handler)

        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Create the API router with MCP endpoint integrated
pub fn create_router_with_mcp(state: AppState) -> Router {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpService, StreamableHttpServerConfig,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let openapi = ApiDoc::openapi();

    // Clone state components for MCP service factory
    let store = state.store.clone();
    let fulltext = state.fulltext.clone();
    let semantic = state.semantic.clone();
    let embedder = state.embedder.clone();
    let chunker = state.chunker.clone();

    let ct = CancellationToken::new();

    let config = StreamableHttpServerConfig {
        cancellation_token: ct,
        ..Default::default()
    };

    let mcp_service = StreamableHttpService::new(
        move || Ok(NotidiumServer::new(store.clone(), fulltext.clone(), semantic.clone(), embedder.clone(), chunker.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    Router::new()
        // Notes CRUD
        .route("/api/notes", get(handlers::list_notes))
        .route("/api/notes", post(handlers::create_note))
        .route("/api/notes/{id}", get(handlers::get_note))
        .route("/api/notes/{id}", put(handlers::update_note))
        .route("/api/notes/{id}", delete(handlers::delete_note))

        // Search
        .route("/api/search", get(handlers::search))
        .route("/api/search/semantic", get(handlers::semantic_search))
        .route("/api/notes/{id}/related", get(handlers::find_related))

        // Quick actions
        .route("/api/capture", post(handlers::quick_capture))

        // Attachments
        .route("/api/attachments", post(handlers::upload_attachment))
        .route("/api/attachments/{filename}", get(handlers::get_attachment))

        // Metadata
        .route("/api/tags", get(handlers::list_tags))
        .route("/api/stats", get(handlers::get_stats))

        // Health
        .route("/health", get(handlers::health))

        // OpenAPI spec and Swagger UI
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", openapi))

        // MCP endpoint
        .nest_service("/mcp", mcp_service)

        // Static files (frontend)
        .fallback(static_handler)

        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
