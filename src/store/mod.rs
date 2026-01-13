//! Storage layer for notes and metadata

mod note_store;
mod metadata_db;
mod manifest;

pub use note_store::NoteStore;
pub use metadata_db::MetadataDb;
pub use manifest::{Manifest, ManifestEntry};
