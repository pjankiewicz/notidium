//! HTTP API layer

mod routes;
mod handlers;

pub use routes::{create_router, create_router_with_mcp, AppState};
