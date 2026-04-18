//! Notidium - Developer-focused, local-first note-taking with semantic search and MCP integration

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use notidium::api::{self, AppState};
use notidium::config::Config;
use notidium::embed::{Chunker, Embedder};
use notidium::mcp::NotidiumServer;
use notidium::search::{FullTextIndex, SemanticSearch};
use notidium::service::{self, ServiceSpec, ServiceState};
use notidium::store::NoteStore;

#[derive(Parser)]
#[command(name = "notidium")]
#[command(about = "Developer-focused, local-first note-taking with semantic search and MCP integration")]
#[command(version)]
struct Cli {
    /// Path to vault directory
    #[arg(long, global = true)]
    vault: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault
    Init {
        /// Path for the new vault
        path: Option<PathBuf>,
    },

    /// Start the HTTP server (includes MCP at /mcp)
    Serve {
        /// Path to vault directory
        path: Option<PathBuf>,

        /// Port to listen on
        #[arg(short, long, default_value = "3939")]
        port: u16,

        /// Disable MCP endpoint
        #[arg(long)]
        no_mcp: bool,
    },

    /// Start the MCP server (stdio mode for Claude Desktop)
    Mcp {
        /// Path to vault directory
        path: Option<PathBuf>,
    },

    /// Start the MCP server (HTTP mode only, no REST API)
    McpHttp {
        /// Path to vault directory
        path: Option<PathBuf>,

        /// Port to listen on
        #[arg(short, long, default_value = "3940")]
        port: u16,
    },

    /// Index all notes
    Index {
        /// Force re-index of all notes
        #[arg(short, long)]
        force: bool,
    },

    /// Search notes
    Search {
        /// Search query
        query: String,

        /// Use semantic search
        #[arg(short, long)]
        semantic: bool,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show vault statistics
    Stats,

    /// List all notes
    List {
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Install the auto-start service (runs notidium serve at login)
    InstallService {
        /// Vault path (defaults to configured vault)
        #[arg(long)]
        vault: Option<PathBuf>,

        /// Port (defaults to configured HTTP port)
        #[arg(short, long)]
        port: Option<u16>,

        /// Overwrite an existing service definition
        #[arg(long)]
        force: bool,
    },

    /// Remove the auto-start service
    UninstallService,

    /// Show the auto-start service state (running/stopped) and recent log lines
    ServiceStatus {
        /// Number of log lines to tail
        #[arg(short = 'n', long, default_value = "20")]
        lines: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            format!("notidium={},tower_http=debug", log_level).into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment
    let _ = dotenvy::dotenv();

    // Load config
    let config = if let Some(vault_path) = &cli.vault {
        Config::load_from_vault(vault_path.clone())?
    } else {
        Config::load()?
    };

    match cli.command {
        Commands::Init { path } => {
            let vault_path = path.unwrap_or_else(|| config.vault_path.clone());
            let mut init_config = config;
            init_config.vault_path = vault_path.clone();

            tracing::info!("Initializing vault at {:?}", vault_path);
            init_config.init_vault()?;
            init_config.save()?;

            println!("✓ Vault initialized at {}", vault_path.display());
            println!("\nNext steps:");
            println!("  1. Add notes to {}/notes/", vault_path.display());
            println!("  2. Run `notidium index` to build the search index");
            println!("  3. Run `notidium serve` to start the API server");
            println!("  4. Run `notidium mcp` to start the MCP server for Claude");
        }

        Commands::Serve { path, port, no_mcp } => {
            let config = resolve_config(config, path, &cli.vault)?;
            let state = initialize_state(&config).await?;

            tracing::info!("Starting HTTP server on port {}", port);

            let router = if no_mcp {
                api::create_router(state)
            } else {
                // Create combined router with both REST API and MCP
                api::create_router_with_mcp(state)
            };

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

            println!("Notidium server running at http://localhost:{}", port);
            println!("  UI:       http://localhost:{}/", port);
            println!("  API:      http://localhost:{}/api/...", port);
            println!("  API Docs: http://localhost:{}/api/docs", port);
            if !no_mcp {
                println!("  MCP:      http://localhost:{}/mcp", port);
            }
            println!("  Health:   http://localhost:{}/health", port);

            axum::serve(listener, router).await?;
        }

        Commands::Mcp { path } => {
            let config = resolve_config(config, path, &cli.vault)?;
            let state = initialize_state(&config).await?;

            tracing::info!("Starting MCP server (stdio mode)");

            let server = NotidiumServer::new(state.store, state.fulltext, state.semantic, state.embedder, state.chunker);

            // Run MCP server over stdio
            notidium::mcp::server::serve_stdio(server).await?;
        }

        Commands::McpHttp { path, port } => {
            let config = resolve_config(config, path, &cli.vault)?;
            let state = initialize_state(&config).await?;

            tracing::info!("Starting MCP server (HTTP mode) on port {}", port);

            let server = NotidiumServer::new(state.store, state.fulltext, state.semantic, state.embedder, state.chunker);

            println!("MCP server running at http://localhost:{}/mcp", port);

            // Run MCP server over HTTP
            notidium::mcp::server::serve_http(server, port).await?;
        }

        Commands::Index { force } => {
            tracing::info!("Indexing notes...");

            let store = NoteStore::new(config.clone());
            let notes = store.load_all().await?;

            println!("Found {} notes", notes.len());

            // Initialize fulltext index
            let fulltext = FullTextIndex::open(&config.tantivy_path())?;
            if force {
                fulltext.rebuild(&notes)?;
            } else {
                for note in &notes {
                    fulltext.index_note(note)?;
                }
                fulltext.commit()?;
            }
            println!("✓ Full-text index updated");

            // Initialize embeddings
            println!("Loading embedding model (this may take a moment on first run)...");
            let embedder = Arc::new(Embedder::new()?);
            let chunker = Chunker::default();

            let mut chunks = Vec::new();
            for note in &notes {
                let note_chunks = chunker.chunk_note(note);
                chunks.extend(note_chunks);
            }
            println!("Generated {} chunks", chunks.len());

            // Embed chunks in batches
            let batch_size = config.embedding.batch_size;
            let total_chunks = chunks.len();
            let mut embedded_count = 0;

            for batch in chunks.chunks_mut(batch_size) {
                let texts: Vec<String> = batch.iter().map(|c| c.content.clone()).collect();
                let embeddings = embedder.embed_batch(texts).await?;

                for (chunk, embedding) in batch.iter_mut().zip(embeddings) {
                    chunk.prose_embedding = Some(embedding);
                    chunk.embedded_at = Some(chrono::Utc::now());
                }

                embedded_count += batch.len();
                println!("  Embedded {}/{} chunks", embedded_count, total_chunks);
            }

            // Save chunks to JSON for now (TODO: use LanceDB)
            let chunks_path = config.data_dir().join("chunks.json");
            let json = serde_json::to_string_pretty(&chunks)?;
            std::fs::write(&chunks_path, json)?;

            println!("✓ Embeddings saved to {}", chunks_path.display());
            println!("\nIndexing complete!");
        }

        Commands::Search { query, semantic, limit } => {
            let state = initialize_state(&config).await?;

            let results = if semantic {
                let sem = state.semantic.read().await;
                sem.search(&query, limit).await?
            } else {
                state.fulltext.search(&query, limit)?
            };

            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("Found {} results:\n", results.len());
                for (i, result) in results.iter().enumerate() {
                    // Get note title
                    let title: String = if let Ok(uuid) = result.note_id.parse::<uuid::Uuid>() {
                        state
                            .store
                            .get(uuid)
                            .await
                            .map(|n| n.title.clone())
                            .unwrap_or_else(|| result.note_id.clone())
                    } else {
                        result.note_id.clone()
                    };

                    println!(
                        "{}. {} (score: {:.3})",
                        i + 1,
                        title,
                        result.score
                    );
                    if !result.snippet.is_empty() {
                        println!("   {}", truncate(&result.snippet, 100));
                    }
                }
            }
        }

        Commands::Stats => {
            let store = NoteStore::new(config.clone());
            let notes = store.load_all().await?;

            let note_count = notes.iter().filter(|n| !n.is_deleted).count();
            let archived_count = notes.iter().filter(|n| n.is_archived).count();

            let mut tags = std::collections::HashSet::new();
            for note in &notes {
                for tag in note.tags() {
                    tags.insert(tag);
                }
            }

            // Check for chunks
            let chunks_path = config.data_dir().join("chunks.json");
            let chunk_count = if chunks_path.exists() {
                let content = std::fs::read_to_string(&chunks_path)?;
                let chunks: Vec<serde_json::Value> = serde_json::from_str(&content)?;
                chunks.len()
            } else {
                0
            };

            println!("Notidium Statistics");
            println!("==================");
            println!("Vault: {}", config.vault_path.display());
            println!();
            println!("Notes:    {}", note_count);
            println!("Archived: {}", archived_count);
            println!("Tags:     {}", tags.len());
            println!("Chunks:   {}", chunk_count);
            println!();

            if !tags.is_empty() {
                let mut sorted: Vec<_> = tags.into_iter().collect();
                sorted.sort();
                println!("Tags: {}", sorted.join(", "));
            }
        }

        Commands::List { limit, tag } => {
            let store = NoteStore::new(config);
            let _ = store.load_all().await?;
            let notes = store.list_paginated(0, limit, tag.as_deref()).await;

            if notes.is_empty() {
                println!("No notes found");
            } else {
                for note in notes {
                    let tags = note.tags();
                    let tag_str = if tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", tags.join(", "))
                    };

                    println!(
                        "• {} ({}){}",
                        note.title,
                        note.updated_at.format("%Y-%m-%d"),
                        tag_str
                    );
                }
            }
        }

        Commands::InstallService { vault, port, force } => {
            let mut cfg = config;
            if let Some(v) = vault {
                cfg = Config::load_from_vault(v)?;
            } else if let Some(v) = &cli.vault {
                cfg = Config::load_from_vault(v.clone())?;
            }

            // A directory is considered an initialized vault when it has a
            // `.notidium/` subdirectory. If so, reuse it as-is (never touch
            // the user's data). Otherwise run init to create the layout.
            if cfg.data_dir().exists() {
                println!("✓ Using existing vault at {}", cfg.vault_path.display());
            } else {
                cfg.init_vault()?;
                cfg.save()?;
                println!("✓ Initialized vault at {}", cfg.vault_path.display());
            }

            let spec = ServiceSpec {
                binary_path: std::env::current_exe()?,
                vault_path: cfg.vault_path.clone(),
                port: port.unwrap_or(cfg.http_port),
                log_path: cfg.logs_path().join("server.log"),
            };

            service::current().install(&spec, force)?;
            println!("✓ Service installed");
            println!("  Binary: {}", spec.binary_path.display());
            println!("  Vault:  {}", spec.vault_path.display());
            println!("  Port:   {}", spec.port);
            println!("  Logs:   {}", spec.log_path.display());
        }

        Commands::UninstallService => {
            service::current().uninstall()?;
            println!("✓ Service removed");
        }

        Commands::ServiceStatus { lines } => {
            let state = service::current().status()?;
            let label = match &state {
                ServiceState::NotInstalled => "not installed",
                ServiceState::Stopped => "stopped",
                ServiceState::Running => "running",
                ServiceState::Failed(_) => "failed",
                ServiceState::Unknown => "unknown",
            };
            println!("Status: {label}");
            if let ServiceState::Failed(msg) = &state {
                println!("  {msg}");
            }

            if !matches!(state, ServiceState::NotInstalled) {
                let log_path = config.logs_path().join("server.log");
                if log_path.exists() {
                    println!("\nLast {lines} lines of {}:", log_path.display());
                    let content = std::fs::read_to_string(&log_path).unwrap_or_default();
                    let tail: Vec<&str> = content.lines().rev().take(lines).collect();
                    for line in tail.into_iter().rev() {
                        println!("  {line}");
                    }
                } else {
                    println!("\nNo log file at {} yet.", log_path.display());
                }
            }
        }
    }

    Ok(())
}

async fn initialize_state(config: &Config) -> anyhow::Result<AppState> {
    // Ensure vault exists
    if !config.vault_path.exists() {
        anyhow::bail!(
            "Vault not found at {}. Run `notidium init` first.",
            config.vault_path.display()
        );
    }

    // Load notes
    let store = Arc::new(NoteStore::new(config.clone()));
    let notes = store.load_all().await?;
    tracing::info!("Loaded {} notes", notes.len());

    // Initialize fulltext index
    let fulltext = Arc::new(FullTextIndex::open(&config.tantivy_path())?);

    // Initialize embedder and chunker
    let embedder = Arc::new(Embedder::new()?);
    let chunker = Arc::new(Chunker::default());

    // Initialize semantic search
    let mut semantic = SemanticSearch::new(embedder.clone());

    // Load chunks if available, filtering out stale chunks whose notes no longer exist
    let chunks_path = config.data_dir().join("chunks.json");
    if chunks_path.exists() {
        let content = std::fs::read_to_string(&chunks_path)?;
        let chunks: Vec<notidium::types::Chunk> = serde_json::from_str(&content)?;
        let total_chunks = chunks.len();

        // Get valid note IDs from the store
        let valid_note_ids: std::collections::HashSet<uuid::Uuid> =
            notes.iter().map(|n| n.id).collect();

        // Filter chunks to only include those with valid note IDs
        let valid_chunks: Vec<_> = chunks
            .into_iter()
            .filter(|c| valid_note_ids.contains(&c.note_id))
            .collect();

        let stale_count = total_chunks - valid_chunks.len();
        if stale_count > 0 {
            tracing::warn!(
                "Filtered out {} stale chunks (notes no longer exist). Run `notidium index -f` to rebuild.",
                stale_count
            );
        }

        semantic.load_chunks(valid_chunks);
        tracing::info!("Loaded {} chunks for semantic search", semantic.chunk_count());
    }

    Ok(AppState {
        store,
        fulltext,
        semantic: Arc::new(RwLock::new(semantic)),
        embedder,
        chunker,
        attachments_path: config.attachments_path(),
    })
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Resolve config with optional path overrides
fn resolve_config(
    mut config: Config,
    path: Option<PathBuf>,
    vault: &Option<PathBuf>,
) -> anyhow::Result<Config> {
    // Path argument takes precedence over --vault flag
    if let Some(p) = path {
        config = Config::load_from_vault(p)?;
    } else if let Some(v) = vault {
        config = Config::load_from_vault(v.clone())?;
    }
    Ok(config)
}
