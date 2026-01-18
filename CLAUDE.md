# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Notidium is a developer-focused, local-first note-taking application with semantic search and native MCP integration. Notes are stored as Markdown files with YAML frontmatter.

## Build Commands

```bash
make setup            # Initial setup - install frontend dependencies
make dev              # Start backend (debug) and frontend dev servers
make dev-release      # Start backend (release) and frontend dev servers
make build            # Build frontend and backend for production
make install          # Build and install notidium CLI globally
make generate-sdk     # Generate TypeScript API client (requires backend running)
```

## Testing & Linting

```bash
cargo test                      # Run all Rust tests
cargo clippy                    # Run Rust linter
cd frontend && npm run lint     # Run ESLint (max-warnings 0)
cd frontend && npm run type-check  # TypeScript type checking
```

## Architecture

**Backend (Rust/Axum):**
- `src/main.rs` - CLI entry point (init, serve, mcp, search, index, list, stats)
- `src/api/handlers.rs` - HTTP endpoint handlers
- `src/api/routes.rs` - Router setup with OpenAPI docs
- `src/store/` - File-based note storage with manifest-based ID tracking
- `src/search/fulltext.rs` - Tantivy full-text search
- `src/search/semantic.rs` - BGE-small-en embedding-based semantic search
- `src/embed/` - Embedding pipeline (chunker + fastembed)
- `src/mcp/` - MCP server for Claude Desktop integration
- `src/types.rs` - Core data types (Note, Chunk, Frontmatter)
- `src/config.rs` - Configuration loading

**Frontend (React/TypeScript/Vite):**
- `frontend/src/pages/` - Main pages (Home, Notes, Search, Tags, Stats)
- `frontend/src/components/` - UI components (MarkdownPreview, CodeBlock, CommandPalette)
- `frontend/src/hooks/` - Data fetching hooks (useNotes, useSearch, useTags)
- `frontend/src/stores/` - Zustand stores (notesStore, settingsStore, uiStore)
- `frontend/src/api/` - Generated OpenAPI client

**Data Flow:**
1. React components use custom hooks for data fetching
2. Hooks use React Query with the generated OpenAPI client
3. Requests go to Axum backend (port 3939)
4. Backend interacts with file system, Tantivy index, and LanceDB vectors

## Key Technical Details

- **TypeScript**: Strict mode enabled, `noImplicitAny: true`. Use of `any` is prohibited.
- **Rust structs**: Avoid `serde_json::Value` as struct properties - use proper types.
- **Vault structure**: Notes stored in `~/Notidium/notes/`, app data in `~/Notidium/.notidium/`
- **Embedded UI**: Frontend is compiled into the binary via rust-embed
- **API generation**: Run `make generate-sdk` after backend API changes

## Development Workflow

1. Run `make dev` to start both servers (frontend on :5173, backend on :3939)
2. Vite proxies `/api` requests to the backend
3. After changing backend API, regenerate the TypeScript client with `make generate-sdk`
