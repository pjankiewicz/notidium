//! Search layer (full-text and semantic)

mod fulltext;
mod semantic;

pub use fulltext::FullTextIndex;
pub use semantic::SemanticSearch;
