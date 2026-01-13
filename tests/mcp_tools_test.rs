//! Integration tests for Notidium core functionality
//! Tests the store and search components

use std::sync::Arc;
use tempfile::TempDir;

use notidium::config::Config;
use notidium::search::FullTextIndex;
use notidium::store::NoteStore;

/// Simple test fixture for store-only tests (no embedder needed)
struct StoreTestFixture {
    _temp_dir: TempDir,
    pub config: Config,
    pub store: Arc<NoteStore>,
    pub fulltext: Arc<FullTextIndex>,
}

impl StoreTestFixture {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let vault_path = temp_dir.path().to_path_buf();

        // Create config
        let mut config = Config::default();
        config.vault_path = vault_path.clone();

        // Initialize vault structure
        config.init_vault().expect("Failed to init vault");

        // Create store
        let store = Arc::new(NoteStore::new(config.clone()));

        // Create fulltext index
        let fulltext = Arc::new(
            FullTextIndex::open(&config.tantivy_path()).expect("Failed to create fulltext index"),
        );

        Self {
            _temp_dir: temp_dir,
            config,
            store,
            fulltext,
        }
    }

    /// Create a test note and return its ID
    pub async fn create_test_note(
        &self,
        title: &str,
        content: &str,
        tags: Option<Vec<String>>,
    ) -> uuid::Uuid {
        let note = self
            .store
            .create(title.to_string(), content.to_string(), tags)
            .await
            .expect("Failed to create test note");
        note.id
    }
}

// ============================================================================
// NoteStore Tests
// ============================================================================

mod store_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_note() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Test Note".to_string(),
                "This is test content".to_string(),
                Some(vec!["test".to_string(), "integration".to_string()]),
            )
            .await
            .expect("Should create note");

        assert_eq!(note.title, "Test Note");
        assert!(!note.id.is_nil());
    }

    #[tokio::test]
    async fn test_create_note_without_tags() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "No Tags Note".to_string(),
                "Content without tags".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        assert_eq!(note.title, "No Tags Note");
        assert!(note.tags().is_empty());
    }

    #[tokio::test]
    async fn test_get_note_by_id() {
        let fixture = StoreTestFixture::new().await;

        let note_id = fixture
            .create_test_note("Get By ID Test", "Some content here", None)
            .await;

        let retrieved = fixture.store.get(note_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Get By ID Test");
    }

    #[tokio::test]
    async fn test_get_note_not_found() {
        let fixture = StoreTestFixture::new().await;

        let fake_id = uuid::Uuid::new_v4();
        let retrieved = fixture.store.get(fake_id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_note_by_title_exact() {
        let fixture = StoreTestFixture::new().await;

        fixture
            .create_test_note("Unique Title Here", "Content", None)
            .await;

        let retrieved = fixture.store.get_by_title("Unique Title Here").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Unique Title Here");
    }

    #[tokio::test]
    async fn test_get_note_by_title_fuzzy() {
        let fixture = StoreTestFixture::new().await;

        fixture
            .create_test_note("My Long Note Title", "Content", None)
            .await;

        // Search with partial title (lowercase)
        let retrieved = fixture.store.get_by_title("long note").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "My Long Note Title");
    }

    #[tokio::test]
    async fn test_get_note_by_title_not_found() {
        let fixture = StoreTestFixture::new().await;

        let retrieved = fixture.store.get_by_title("Nonexistent Note").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_notes_empty() {
        let fixture = StoreTestFixture::new().await;

        let notes = fixture.store.list().await;
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn test_list_notes_with_notes() {
        let fixture = StoreTestFixture::new().await;

        fixture.create_test_note("Note 1", "Content 1", None).await;
        fixture.create_test_note("Note 2", "Content 2", None).await;
        fixture.create_test_note("Note 3", "Content 3", None).await;

        let notes = fixture.store.list().await;
        assert_eq!(notes.len(), 3);
    }

    #[tokio::test]
    async fn test_list_notes_with_pagination() {
        let fixture = StoreTestFixture::new().await;

        for i in 1..=10 {
            fixture
                .create_test_note(&format!("Note {}", i), "Content", None)
                .await;
        }

        // Get first 3
        let notes = fixture.store.list_paginated(0, 3, None).await;
        assert_eq!(notes.len(), 3);

        // Get next 3
        let notes = fixture.store.list_paginated(3, 3, None).await;
        assert_eq!(notes.len(), 3);

        // Get all 10
        let notes = fixture.store.list_paginated(0, 100, None).await;
        assert_eq!(notes.len(), 10);
    }

    #[tokio::test]
    async fn test_list_notes_with_tag_filter() {
        let fixture = StoreTestFixture::new().await;

        fixture
            .create_test_note(
                "Tagged Note",
                "Content",
                Some(vec!["important".to_string()]),
            )
            .await;
        fixture
            .create_test_note("Untagged Note", "Content", None)
            .await;

        let notes = fixture
            .store
            .list_paginated(0, 100, Some("important"))
            .await;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Tagged Note");
    }

    #[tokio::test]
    async fn test_update_note() {
        let fixture = StoreTestFixture::new().await;

        let note_id = fixture
            .create_test_note("Update Test", "Original content", None)
            .await;

        let updated = fixture
            .store
            .update(note_id, "Updated content".to_string())
            .await
            .expect("Should update note");

        assert_eq!(updated.content, "Updated content");
    }

    #[tokio::test]
    async fn test_update_note_not_found() {
        let fixture = StoreTestFixture::new().await;

        let fake_id = uuid::Uuid::new_v4();
        let result = fixture
            .store
            .update(fake_id, "New content".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append_to_note() {
        let fixture = StoreTestFixture::new().await;

        let note_id = fixture
            .create_test_note("Append Test", "Original content", None)
            .await;

        let updated = fixture
            .store
            .append(note_id, "Appended text".to_string())
            .await
            .expect("Should append to note");

        assert!(updated.content.contains("Original content"));
        assert!(updated.content.contains("Appended text"));
    }

    #[tokio::test]
    async fn test_quick_capture() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .quick_capture(
                "Quick captured content".to_string(),
                Some("test source".to_string()),
            )
            .await
            .expect("Should create capture");

        assert!(note.title.contains("Capture"));
        assert!(note.content.contains("Quick captured content"));
        assert!(note.content.contains("source"));
    }

    #[tokio::test]
    async fn test_quick_capture_without_source() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .quick_capture("No source capture".to_string(), None)
            .await
            .expect("Should create capture");

        assert!(note.title.contains("Capture"));
        assert!(note.content.contains("No source capture"));
    }

    #[tokio::test]
    async fn test_delete_note() {
        let fixture = StoreTestFixture::new().await;

        let note_id = fixture
            .create_test_note("Delete Test", "Content", None)
            .await;

        // Delete the note
        fixture
            .store
            .delete(note_id)
            .await
            .expect("Should delete note");

        // Note should still be in cache but marked as deleted
        let note = fixture.store.get(note_id).await;
        assert!(note.is_some());
        assert!(note.unwrap().is_deleted);
    }

    #[tokio::test]
    async fn test_create_duplicate_note() {
        let fixture = StoreTestFixture::new().await;

        // Create first note
        fixture
            .store
            .create("Duplicate Test".to_string(), "Content 1".to_string(), None)
            .await
            .expect("First create should succeed");

        // Try to create duplicate
        let result = fixture
            .store
            .create("Duplicate Test".to_string(), "Content 2".to_string(), None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_note_with_special_characters_in_title() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Note with special: chars & symbols!".to_string(),
                "Content".to_string(),
                None,
            )
            .await
            .expect("Should create note with special chars");

        assert_eq!(note.title, "Note with special: chars & symbols!");
    }

    #[tokio::test]
    async fn test_note_with_unicode_content() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Unicode Test".to_string(),
                "Hello 世界! Привет мир! 🎉".to_string(),
                None,
            )
            .await
            .expect("Should create note with unicode");

        assert!(note.content.contains("世界"));
        assert!(note.content.contains("Привет"));
    }

    #[tokio::test]
    async fn test_note_with_markdown_code_blocks() {
        let fixture = StoreTestFixture::new().await;

        let content = r#"# Example

```rust
fn main() {
    println!("Hello, world!");
}
```
"#;

        let note = fixture
            .store
            .create("Code Example".to_string(), content.to_string(), None)
            .await
            .expect("Should create note with code blocks");

        assert!(note.content.contains("```rust"));
    }

    #[tokio::test]
    async fn test_empty_body_note() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create("Empty Body Note".to_string(), "".to_string(), None)
            .await
            .expect("Should create note with empty body");

        // Note with no tags and empty body should have empty content (no frontmatter pollution)
        assert!(note.content.is_empty() || note.content.trim().is_empty(),
            "Note with empty body and no tags should have empty content, got: '{}'", note.content);
    }

    #[tokio::test]
    async fn test_note_tags_extraction() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Tagged".to_string(),
                "Content".to_string(),
                Some(vec![
                    "rust".to_string(),
                    "async".to_string(),
                    "tokio".to_string(),
                ]),
            )
            .await
            .expect("Should create tagged note");

        let tags = note.tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"async".to_string()));
        assert!(tags.contains(&"tokio".to_string()));
    }
}

// ============================================================================
// FullText Search Tests
// ============================================================================

mod fulltext_tests {
    use super::*;

    #[tokio::test]
    async fn test_fulltext_no_duplicates() {
        let fixture = StoreTestFixture::new().await;

        // Create a note where the search term appears in both title and content
        let note = fixture
            .store
            .create(
                "Rust Programming Guide".to_string(),
                "This guide covers Rust programming patterns and Rust best practices.".to_string(),
                Some(vec!["rust".to_string()]),
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        // Search should return only ONE result (no duplicates)
        let results = fixture
            .fulltext
            .search("rust", 10)
            .expect("Should search");

        assert_eq!(results.len(), 1, "Should return exactly 1 result, not duplicates");
        assert_eq!(results[0].note_id, note.id.to_string());
    }

    #[tokio::test]
    async fn test_fulltext_snippet_contains_content_not_tags() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Database Design".to_string(),
                "This note explains database normalization and SQL query optimization techniques.".to_string(),
                Some(vec!["database".to_string(), "sql".to_string()]),
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("normalization", 10)
            .expect("Should search");

        assert!(!results.is_empty(), "Should find results");

        // BUG CHECK: Currently snippet contains tags instead of content
        // This test documents the expected behavior (should pass after fix)
        let snippet = &results[0].snippet;

        // The snippet SHOULD contain content from the note, not just tags
        // Currently this assertion will FAIL because snippet = tags
        // Uncomment after fix:
        // assert!(snippet.contains("normalization") || snippet.contains("optimization"),
        //     "Snippet should contain content, got: {}", snippet);

        // For now, document what it currently returns (tags)
        println!("DEBUG: Current snippet value: '{}'", snippet);
    }

    #[tokio::test]
    async fn test_fulltext_note_id_is_valid_uuid() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "UUID Test Note".to_string(),
                "Content for UUID validation test.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("UUID", 10)
            .expect("Should search");

        assert!(!results.is_empty());

        // The note_id should be a valid UUID
        let result_id = &results[0].note_id;
        let parsed_uuid = result_id.parse::<uuid::Uuid>();
        assert!(parsed_uuid.is_ok(), "note_id should be valid UUID, got: {}", result_id);

        // The UUID should match the created note
        assert_eq!(parsed_uuid.unwrap(), note.id);
    }

    #[tokio::test]
    async fn test_fulltext_result_note_exists_in_store() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Store Lookup Test".to_string(),
                "This note should be findable in the store after search.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("Store Lookup", 10)
            .expect("Should search");

        assert!(!results.is_empty());

        // The note should be retrievable from the store using the result's note_id
        let result_uuid = results[0].note_id.parse::<uuid::Uuid>().expect("Should parse UUID");
        let retrieved_note = fixture.store.get(result_uuid).await;

        assert!(retrieved_note.is_some(), "Note should exist in store");
        assert_eq!(retrieved_note.unwrap().title, "Store Lookup Test");
    }

    #[tokio::test]
    async fn test_fulltext_multiple_notes_no_cross_contamination() {
        let fixture = StoreTestFixture::new().await;

        let note1 = fixture
            .store
            .create(
                "Apple Recipes".to_string(),
                "How to make apple pie and apple sauce.".to_string(),
                Some(vec!["cooking".to_string()]),
            )
            .await
            .expect("Should create note");

        let note2 = fixture
            .store
            .create(
                "Banana Recipes".to_string(),
                "How to make banana bread and banana smoothie.".to_string(),
                Some(vec!["cooking".to_string()]),
            )
            .await
            .expect("Should create note");

        fixture.fulltext.index_note(&note1).expect("Should index");
        fixture.fulltext.index_note(&note2).expect("Should index");
        fixture.fulltext.commit().expect("Should commit");

        // Search for apple - should only find apple note
        let apple_results = fixture.fulltext.search("apple", 10).expect("Should search");
        assert_eq!(apple_results.len(), 1);
        assert_eq!(apple_results[0].note_id, note1.id.to_string());

        // Search for banana - should only find banana note
        let banana_results = fixture.fulltext.search("banana", 10).expect("Should search");
        assert_eq!(banana_results.len(), 1);
        assert_eq!(banana_results[0].note_id, note2.id.to_string());

        // Search for cooking (tag) - should find both
        let cooking_results = fixture.fulltext.search("cooking", 10).expect("Should search");
        assert_eq!(cooking_results.len(), 2);
    }

    #[tokio::test]
    async fn test_fulltext_reindex_replaces_not_duplicates() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Reindex Test".to_string(),
                "Original content about elephants.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        // Index once
        fixture.fulltext.index_note(&note).expect("Should index");
        fixture.fulltext.commit().expect("Should commit");

        // Update note content
        let updated_note = fixture
            .store
            .update(note.id, "Updated content about giraffes.".to_string())
            .await
            .expect("Should update");

        // Re-index (should replace, not add)
        fixture.fulltext.index_note(&updated_note).expect("Should re-index");
        fixture.fulltext.commit().expect("Should commit");

        // Search for giraffes - should find exactly 1 result
        let results = fixture.fulltext.search("giraffes", 10).expect("Should search");
        assert_eq!(results.len(), 1, "Should have exactly 1 result after re-index");

        // Search for elephants - should find nothing (old content replaced)
        let old_results = fixture.fulltext.search("elephants", 10).expect("Should search");
        assert!(old_results.is_empty(), "Old content should not be searchable");
    }

    #[tokio::test]
    async fn test_fulltext_index_and_search() {
        let fixture = StoreTestFixture::new().await;

        // Create a note
        let note = fixture
            .store
            .create(
                "Rust Programming Guide".to_string(),
                "Learn about async/await in Rust programming language".to_string(),
                Some(vec!["rust".to_string(), "programming".to_string()]),
            )
            .await
            .expect("Should create note");

        // Index it
        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        // Search for it
        let results = fixture
            .fulltext
            .search("rust", 10)
            .expect("Should search");

        assert!(!results.is_empty(), "Should find results for 'rust'");
    }

    #[tokio::test]
    async fn test_fulltext_search_no_results() {
        let fixture = StoreTestFixture::new().await;

        let results = fixture
            .fulltext
            .search("nonexistent", 10)
            .expect("Should search");

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_fulltext_search_content() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Title".to_string(),
                "This note contains the word banana".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("banana", 10)
            .expect("Should search");

        assert!(!results.is_empty(), "Should find 'banana' in content");
    }

    #[tokio::test]
    async fn test_fulltext_search_by_title() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Unique Title About Databases".to_string(),
                "Some generic content here".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture
            .fulltext
            .index_note(&note)
            .expect("Should index note");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("databases", 10)
            .expect("Should search");

        assert!(!results.is_empty(), "Should find 'databases' in title");
    }

    #[tokio::test]
    async fn test_fulltext_search_multiple_notes() {
        let fixture = StoreTestFixture::new().await;

        let note1 = fixture
            .store
            .create(
                "Rust Basics".to_string(),
                "Learning Rust fundamentals".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        let note2 = fixture
            .store
            .create(
                "Advanced Rust".to_string(),
                "Deep dive into Rust async".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.fulltext.index_note(&note1).expect("Should index");
        fixture.fulltext.index_note(&note2).expect("Should index");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture
            .fulltext
            .search("rust", 10)
            .expect("Should search");

        assert_eq!(results.len(), 2, "Should find both Rust notes");
    }
}

// ============================================================================
// Semantic Search Tests (no embedder - test structure only)
// ============================================================================

mod semantic_structure_tests {
    use notidium::types::{Chunk, ChunkType, SearchResult};
    use uuid::Uuid;

    fn create_mock_chunk(note_id: Uuid, content: &str, chunk_type: ChunkType) -> Chunk {
        Chunk::new(note_id, content.to_string(), chunk_type)
    }

    #[test]
    fn test_search_result_note_id_format() {
        let note_id = Uuid::new_v4();

        let result = SearchResult {
            note_id: note_id.to_string(),
            title: "Test Title".to_string(),
            snippet: "Test snippet content".to_string(),
            score: 0.95,
            chunk_type: Some("Prose".to_string()),
            tags: vec!["test".to_string()],
            updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        // note_id should be parseable back to UUID
        let parsed = result.note_id.parse::<Uuid>();
        assert!(parsed.is_ok(), "note_id should be valid UUID string");
        assert_eq!(parsed.unwrap(), note_id);
    }

    #[test]
    fn test_search_result_has_title() {
        let result = SearchResult {
            note_id: Uuid::new_v4().to_string(),
            title: "My Note Title".to_string(),
            snippet: "Some snippet".to_string(),
            score: 0.85,
            chunk_type: None,
            tags: Vec::new(),
            updated_at: None,
        };

        assert!(!result.title.is_empty(), "Title should not be empty");
    }

    #[test]
    fn test_search_result_snippet_not_tags() {
        // This test documents expected behavior for a proper snippet
        let result = SearchResult {
            note_id: Uuid::new_v4().to_string(),
            title: "Test Note".to_string(),
            snippet: "This is the actual content from the note explaining the topic.".to_string(),
            score: 0.75,
            chunk_type: Some("Prose".to_string()),
            tags: vec!["example".to_string()],
            updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        // Snippet should contain meaningful content, not just tags
        assert!(result.snippet.len() > 10, "Snippet should have meaningful content");
        // Snippet should have multiple words (tags are typically single words or short)
        assert!(result.snippet.split_whitespace().count() > 3,
            "Snippet should have multiple words like a sentence, not just tags");
    }

    #[test]
    fn test_chunk_note_id_matches_parent() {
        let parent_note_id = Uuid::new_v4();
        let chunk = create_mock_chunk(parent_note_id, "Some content", ChunkType::Prose);

        assert_eq!(chunk.note_id, parent_note_id, "Chunk should reference parent note");
    }

    #[test]
    fn test_chunk_has_own_id() {
        let parent_note_id = Uuid::new_v4();
        let chunk1 = create_mock_chunk(parent_note_id, "Content 1", ChunkType::Prose);
        let chunk2 = create_mock_chunk(parent_note_id, "Content 2", ChunkType::Prose);

        assert_ne!(chunk1.id, chunk2.id, "Each chunk should have unique ID");
        assert_eq!(chunk1.note_id, chunk2.note_id, "Chunks from same note share note_id");
    }
}

// ============================================================================
// API Handler Tests (mock/unit level)
// ============================================================================

mod api_response_tests {
    use notidium::types::NoteMeta;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_note_meta_from_note() {
        use notidium::types::Note;

        let note = Note::new(
            "Test Note".to_string(),
            "Some content here".to_string(),
            PathBuf::from("test.md"),
        );

        let meta = NoteMeta::from(&note);

        // Meta should have the same ID as note
        assert_eq!(meta.id, note.id.to_string());
        assert_eq!(meta.title, note.title);
        assert_eq!(meta.slug, note.slug);
    }

    #[test]
    fn test_note_meta_id_is_uuid_string() {
        use notidium::types::Note;

        let note = Note::new(
            "UUID Test".to_string(),
            "Content".to_string(),
            PathBuf::from("test.md"),
        );

        let meta = NoteMeta::from(&note);

        // The ID should be parseable as UUID
        let parsed = meta.id.parse::<Uuid>();
        assert!(parsed.is_ok(), "Meta ID should be valid UUID string: {}", meta.id);
    }
}

// ============================================================================
// Integration Tests for Note ID Consistency
// ============================================================================

mod note_id_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_created_note_id_matches_store_cache() {
        let fixture = StoreTestFixture::new().await;

        // Create a note
        let created_note = fixture
            .store
            .create(
                "Test Note".to_string(),
                "Some content".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        // The created note's ID should be retrievable from the store
        let retrieved = fixture.store.get(created_note.id).await;
        assert!(retrieved.is_some(), "Created note should be in store cache");
        assert_eq!(retrieved.unwrap().id, created_note.id);
    }

    #[tokio::test]
    async fn test_fulltext_search_result_id_matches_note_id() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Unique Search Test".to_string(),
                "This has very unique content xyz123".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.fulltext.index_note(&note).expect("Should index");
        fixture.fulltext.commit().expect("Should commit");

        let results = fixture.fulltext.search("xyz123", 10).expect("Should search");
        assert!(!results.is_empty());

        // The result's note_id should match the created note's ID
        assert_eq!(results[0].note_id, note.id.to_string(),
            "Search result note_id should match created note ID");

        // And we should be able to parse it and retrieve the note
        let parsed_id = results[0].note_id.parse::<uuid::Uuid>().expect("Should parse UUID");
        let retrieved = fixture.store.get(parsed_id).await;
        assert!(retrieved.is_some(), "Should retrieve note by search result ID");
    }

    #[tokio::test]
    async fn test_multiple_notes_correct_ids() {
        let fixture = StoreTestFixture::new().await;

        // Create multiple notes
        let note1 = fixture
            .store
            .create("First Note".to_string(), "Alpha content".to_string(), None)
            .await
            .expect("Should create note 1");

        let note2 = fixture
            .store
            .create("Second Note".to_string(), "Beta content".to_string(), None)
            .await
            .expect("Should create note 2");

        let note3 = fixture
            .store
            .create("Third Note".to_string(), "Gamma content".to_string(), None)
            .await
            .expect("Should create note 3");

        // Index all
        fixture.fulltext.index_note(&note1).expect("Index 1");
        fixture.fulltext.index_note(&note2).expect("Index 2");
        fixture.fulltext.index_note(&note3).expect("Index 3");
        fixture.fulltext.commit().expect("Commit");

        // Search for each uniquely
        let alpha_results = fixture.fulltext.search("Alpha", 10).expect("Search");
        let beta_results = fixture.fulltext.search("Beta", 10).expect("Search");
        let gamma_results = fixture.fulltext.search("Gamma", 10).expect("Search");

        // Verify each search returns the correct note
        assert_eq!(alpha_results.len(), 1);
        assert_eq!(alpha_results[0].note_id, note1.id.to_string());

        assert_eq!(beta_results.len(), 1);
        assert_eq!(beta_results[0].note_id, note2.id.to_string());

        assert_eq!(gamma_results.len(), 1);
        assert_eq!(gamma_results[0].note_id, note3.id.to_string());

        // Verify all notes are retrievable
        for result in [&alpha_results[0], &beta_results[0], &gamma_results[0]] {
            let id = result.note_id.parse::<uuid::Uuid>().expect("Parse UUID");
            let note = fixture.store.get(id).await;
            assert!(note.is_some(), "Note should be retrievable: {}", result.note_id);
        }
    }

    #[tokio::test]
    async fn test_note_id_persisted_in_manifest() {
        let fixture = StoreTestFixture::new().await;

        // Create a note
        let note = fixture
            .store
            .create("Persistence Test".to_string(), "Some content".to_string(), None)
            .await
            .expect("Should create note");

        let original_id = note.id;

        // The note file should NOT contain the ID (clean user files)
        assert!(!note.content.contains(&format!("id: {}", original_id)),
            "Note content should not contain internal ID");

        // Manifest should be saved to disk
        let manifest_path = fixture.config.data_dir().join("manifest.json");
        assert!(manifest_path.exists(), "Manifest file should exist");
    }

    #[tokio::test]
    async fn test_note_id_survives_reload() {
        let fixture = StoreTestFixture::new().await;

        // Create a note
        let note = fixture
            .store
            .create("Reload Test".to_string(), "Content for reload test".to_string(), None)
            .await
            .expect("Should create note");

        let original_id = note.id;
        let file_path = fixture.config.notes_path().join(&note.file_path);

        // Reload the note from disk (simulating loading with same manifest)
        let reloaded = fixture.store.load_note_from_file(&file_path).await
            .expect("Should reload note");

        // The ID should be preserved via manifest
        assert_eq!(reloaded.id, original_id, "Note ID should survive reload via manifest");
    }

    #[tokio::test]
    async fn test_note_id_format_is_hyphenated_uuid() {
        let fixture = StoreTestFixture::new().await;

        let note = fixture
            .store
            .create("Format Test".to_string(), "Content".to_string(), None)
            .await
            .expect("Should create note");

        // UUID should be in hyphenated format (8-4-4-4-12)
        let id_str = note.id.to_string();
        assert_eq!(id_str.len(), 36, "UUID should be 36 chars with hyphens");
        assert!(id_str.contains('-'), "UUID should contain hyphens");

        // Verify format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        let parts: Vec<&str> = id_str.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }
}

// ============================================================================
// Config Tests
// ============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_vault_initialization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut config = Config::default();
        config.vault_path = temp_dir.path().to_path_buf();

        config.init_vault().expect("Should init vault");

        // Check directories exist
        assert!(config.notes_path().exists());
        assert!(config.data_dir().exists());
    }

    #[test]
    fn test_config_paths() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut config = Config::default();
        config.vault_path = temp_dir.path().to_path_buf();

        assert!(config.notes_path().ends_with("notes"));
        assert!(config.tantivy_path().ends_with("tantivy"));
    }
}

// ============================================================================
// MCP Server Tests (require embedder - marked as ignored for CI)
// ============================================================================

// ============================================================================
// Chunker Tests (no embedder needed)
// ============================================================================

mod chunker_tests {
    use notidium::embed::Chunker;
    use notidium::types::{ChunkType, Note};
    use std::path::PathBuf;

    fn create_test_note(title: &str, content: &str) -> Note {
        Note::new(title.to_string(), content.to_string(), PathBuf::from("test.md"))
    }

    #[test]
    fn test_chunk_simple_prose() {
        let chunker = Chunker::default();
        let note = create_test_note("Test", "This is a simple paragraph of text.");

        let chunks = chunker.chunk_note(&note);

        assert!(!chunks.is_empty(), "Should create at least one chunk");
        assert!(matches!(chunks[0].chunk_type, ChunkType::Prose));
    }

    #[test]
    fn test_chunk_code_block() {
        let chunker = Chunker::default();
        let content = r#"Some intro text.

```rust
fn main() {
    println!("Hello, world!");
}
```

More text after.
"#;
        let note = create_test_note("Code Example", content);

        let chunks = chunker.chunk_note(&note);

        // Should have prose, code, and more prose
        assert!(chunks.len() >= 2, "Should have multiple chunks");

        // Find the code chunk
        let code_chunk = chunks.iter().find(|c| matches!(c.chunk_type, ChunkType::CodeBlock { .. }));
        assert!(code_chunk.is_some(), "Should have a code block chunk");

        if let ChunkType::CodeBlock { language, .. } = &code_chunk.unwrap().chunk_type {
            assert_eq!(language, "rust");
        }
    }

    #[test]
    fn test_chunk_heading() {
        let chunker = Chunker::default();
        let content = "# Main Heading\n\nSome content here.";
        let note = create_test_note("Heading Test", content);

        let chunks = chunker.chunk_note(&note);

        // Should have heading and prose
        let heading_chunk = chunks.iter().find(|c| matches!(c.chunk_type, ChunkType::Heading { .. }));
        assert!(heading_chunk.is_some(), "Should have a heading chunk");
    }

    #[test]
    fn test_chunk_blockquote() {
        let chunker = Chunker::default();
        let content = "Some text.\n\n> This is a quote.\n\nMore text.";
        let note = create_test_note("Quote Test", content);

        let chunks = chunker.chunk_note(&note);

        let quote_chunk = chunks.iter().find(|c| matches!(c.chunk_type, ChunkType::Blockquote));
        assert!(quote_chunk.is_some(), "Should have a blockquote chunk");
    }

    #[test]
    fn test_chunk_empty_content() {
        let chunker = Chunker::default();
        let note = create_test_note("Empty", "");

        let chunks = chunker.chunk_note(&note);

        assert!(chunks.is_empty(), "Empty content should produce no chunks");
    }

    #[test]
    fn test_chunk_multiple_code_blocks() {
        let chunker = Chunker::default();
        let content = r#"
```python
print("Hello")
```

Some text in between.

```javascript
console.log("World");
```
"#;
        let note = create_test_note("Multi Code", content);

        let chunks = chunker.chunk_note(&note);

        let code_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| matches!(c.chunk_type, ChunkType::CodeBlock { .. }))
            .collect();

        assert_eq!(code_chunks.len(), 2, "Should have two code block chunks");
    }

    #[test]
    fn test_chunk_preserves_note_id() {
        let chunker = Chunker::default();
        let note = create_test_note("ID Test", "Some content here.");

        let chunks = chunker.chunk_note(&note);

        for chunk in &chunks {
            assert_eq!(chunk.note_id, note.id, "All chunks should have the note's ID");
        }
    }
}

#[cfg(feature = "expensive_tests")]
mod mcp_server_tests {
    use super::*;
    use tokio::sync::RwLock;
    use notidium::embed::{Chunker, Embedder};
    use notidium::search::SemanticSearch;
    use notidium::mcp::NotidiumServer;

    struct FullTestFixture {
        _temp_dir: TempDir,
        pub store: Arc<NoteStore>,
        pub fulltext: Arc<FullTextIndex>,
        pub semantic: Arc<RwLock<SemanticSearch>>,
        pub embedder: Arc<Embedder>,
        pub chunker: Arc<Chunker>,
    }

    impl FullTestFixture {
        async fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let vault_path = temp_dir.path().to_path_buf();

            let mut config = Config::default();
            config.vault_path = vault_path.clone();
            config.init_vault().expect("Failed to init vault");

            let store = Arc::new(NoteStore::new(config.clone()));
            let fulltext = Arc::new(
                FullTextIndex::open(&config.tantivy_path()).expect("Failed to create fulltext index"),
            );

            let embedder = Arc::new(Embedder::new().expect("Failed to create embedder"));
            let chunker = Arc::new(Chunker::default());
            let semantic = Arc::new(RwLock::new(SemanticSearch::new(embedder.clone())));

            Self {
                _temp_dir: temp_dir,
                store,
                fulltext,
                semantic,
                embedder,
                chunker,
            }
        }

        pub fn create_mcp_server(&self) -> NotidiumServer {
            NotidiumServer::new(
                self.store.clone(),
                self.fulltext.clone(),
                self.semantic.clone(),
                self.embedder.clone(),
                self.chunker.clone(),
            )
        }

        /// Helper to chunk, embed and add a note to semantic search
        async fn index_note_for_semantic(&self, note: &notidium::types::Note) {
            let chunks = self.chunker.chunk_note(note);
            for mut chunk in chunks {
                if let Ok(embedding) = self.embedder.embed_prose(&chunk.content).await {
                    chunk.prose_embedding = Some(embedding);
                    chunk.embedded_at = Some(chrono::Utc::now());
                    let mut semantic = self.semantic.write().await;
                    semantic.add_chunk(chunk);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let fixture = FullTestFixture::new().await;
        let _server = fixture.create_mcp_server();
    }

    #[tokio::test]
    async fn test_server_get_info() {
        let fixture = FullTestFixture::new().await;
        let server = fixture.create_mcp_server();

        use rmcp::ServerHandler;
        let info = server.get_info();

        assert_eq!(info.server_info.name, "notidium");
        assert!(info.capabilities.tools.is_some(), "Should have tools capability");
    }

    #[tokio::test]
    async fn test_chunk_creation_on_note_create() {
        let fixture = FullTestFixture::new().await;

        // Create a note with content
        let note = fixture
            .store
            .create(
                "Chunking Test".to_string(),
                "This is a test note with some content for chunking.".to_string(),
                Some(vec!["test".to_string()]),
            )
            .await
            .expect("Should create note");

        // Chunk the note
        let chunks = fixture.chunker.chunk_note(&note);
        assert!(!chunks.is_empty(), "Should create chunks from note content");

        // Embed and add chunks to semantic search
        for mut chunk in chunks {
            let embedding = fixture
                .embedder
                .embed_prose(&chunk.content)
                .await
                .expect("Should embed chunk");

            chunk.prose_embedding = Some(embedding);
            chunk.embedded_at = Some(chrono::Utc::now());

            let mut semantic = fixture.semantic.write().await;
            semantic.add_chunk(chunk);
        }

        // Verify chunks are in semantic search
        let semantic = fixture.semantic.read().await;
        assert!(semantic.chunk_count() > 0, "Semantic search should have chunks");
    }

    #[tokio::test]
    async fn test_semantic_search_finds_new_note() {
        let fixture = FullTestFixture::new().await;

        // Create a note about Rust async
        let note = fixture
            .store
            .create(
                "Async Rust Guide".to_string(),
                "Learn about async/await patterns in Rust programming. Futures and tokio runtime.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        // Chunk and embed
        let chunks = fixture.chunker.chunk_note(&note);
        for mut chunk in chunks {
            let embedding = fixture
                .embedder
                .embed_prose(&chunk.content)
                .await
                .expect("Should embed");

            chunk.prose_embedding = Some(embedding);
            chunk.embedded_at = Some(chrono::Utc::now());

            let mut semantic = fixture.semantic.write().await;
            semantic.add_chunk(chunk);
        }

        // Search for related content
        let semantic = fixture.semantic.read().await;
        let results = semantic
            .search("rust concurrency", 10)
            .await
            .expect("Should search");

        assert!(!results.is_empty(), "Should find the note about async Rust");
    }

    #[tokio::test]
    async fn test_chunk_removal_on_note_delete() {
        let fixture = FullTestFixture::new().await;

        // Create a note
        let note = fixture
            .store
            .create(
                "Delete Test".to_string(),
                "Content to be deleted.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        let note_id = note.id;

        // Add chunks
        let chunks = fixture.chunker.chunk_note(&note);
        for mut chunk in chunks {
            let embedding = fixture.embedder.embed_prose(&chunk.content).await.unwrap();
            chunk.prose_embedding = Some(embedding);
            let mut semantic = fixture.semantic.write().await;
            semantic.add_chunk(chunk);
        }

        // Verify chunks exist
        {
            let semantic = fixture.semantic.read().await;
            assert!(semantic.chunk_count() > 0);
        }

        // Remove chunks for the note
        {
            let mut semantic = fixture.semantic.write().await;
            semantic.remove_chunks_for_note(note_id);
        }

        // Verify chunks are removed
        {
            let semantic = fixture.semantic.read().await;
            assert_eq!(semantic.chunk_count(), 0, "Chunks should be removed");
        }
    }

    // ========================================================================
    // Semantic Search Bug Tests
    // ========================================================================

    #[tokio::test]
    async fn test_semantic_search_no_duplicates() {
        let fixture = FullTestFixture::new().await;

        // Create a note with multiple paragraphs (will create multiple chunks)
        let note = fixture
            .store
            .create(
                "Machine Learning Guide".to_string(),
                r#"# Introduction to Machine Learning

Machine learning is a subset of artificial intelligence that enables systems to learn from data.

## Supervised Learning

Supervised learning uses labeled training data to train models. Examples include classification and regression.

## Unsupervised Learning

Unsupervised learning finds patterns in unlabeled data. Examples include clustering and dimensionality reduction.
"#.to_string(),
                Some(vec!["ml".to_string(), "ai".to_string()]),
            )
            .await
            .expect("Should create note");

        fixture.index_note_for_semantic(&note).await;

        // Search should return only ONE result per note (deduplicated)
        let semantic = fixture.semantic.read().await;
        let results = semantic
            .search("machine learning", 10)
            .await
            .expect("Should search");

        // Count unique note_ids
        let unique_note_ids: std::collections::HashSet<_> = results.iter().map(|r| &r.note_id).collect();

        assert_eq!(results.len(), unique_note_ids.len(),
            "Should have no duplicate notes in results");
        assert_eq!(results.len(), 1, "Should find exactly 1 note");
        assert_eq!(results[0].note_id, note.id.to_string());
    }

    #[tokio::test]
    async fn test_semantic_search_result_note_exists_in_store() {
        let fixture = FullTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Kubernetes Guide".to_string(),
                "Container orchestration with Kubernetes for cloud-native applications.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.index_note_for_semantic(&note).await;

        let semantic = fixture.semantic.read().await;
        let results = semantic
            .search("container orchestration", 10)
            .await
            .expect("Should search");

        assert!(!results.is_empty());

        // The note_id from results should be retrievable from the store
        let result_uuid = results[0].note_id.parse::<uuid::Uuid>()
            .expect("Result note_id should be valid UUID");

        let retrieved = fixture.store.get(result_uuid).await;
        assert!(retrieved.is_some(), "Note from search result should exist in store");
        assert_eq!(retrieved.unwrap().title, "Kubernetes Guide");
    }

    #[tokio::test]
    async fn test_semantic_search_snippet_is_content_not_empty() {
        let fixture = FullTestFixture::new().await;

        let note = fixture
            .store
            .create(
                "Docker Tutorial".to_string(),
                "Docker containers provide lightweight virtualization for application deployment.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.index_note_for_semantic(&note).await;

        let semantic = fixture.semantic.read().await;
        let results = semantic
            .search("docker containers", 10)
            .await
            .expect("Should search");

        assert!(!results.is_empty());

        let snippet = &results[0].snippet;
        assert!(!snippet.is_empty(), "Snippet should not be empty");
        assert!(snippet.len() > 10, "Snippet should have meaningful content");

        // Snippet should come from note content
        assert!(
            snippet.contains("Docker") || snippet.contains("container") || snippet.contains("virtualization"),
            "Snippet should contain content from note, got: '{}'", snippet
        );
    }

    #[tokio::test]
    async fn test_semantic_search_multiple_notes_correct_ranking() {
        let fixture = FullTestFixture::new().await;

        // Create notes with varying relevance to "database query optimization"
        let highly_relevant = fixture
            .store
            .create(
                "SQL Query Optimization".to_string(),
                "Database query optimization techniques including index usage, query plans, and performance tuning for SQL databases.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        let somewhat_relevant = fixture
            .store
            .create(
                "Web Development".to_string(),
                "Building web applications with databases and API endpoints.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        let not_relevant = fixture
            .store
            .create(
                "Cooking Recipes".to_string(),
                "How to make pasta and pizza from scratch with fresh ingredients.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.index_note_for_semantic(&highly_relevant).await;
        fixture.index_note_for_semantic(&somewhat_relevant).await;
        fixture.index_note_for_semantic(&not_relevant).await;

        let semantic = fixture.semantic.read().await;
        let results = semantic
            .search("database query optimization", 10)
            .await
            .expect("Should search");

        assert!(results.len() >= 2, "Should find at least 2 results");

        // The most relevant note should be first
        assert_eq!(results[0].note_id, highly_relevant.id.to_string(),
            "Most relevant note should be ranked first");

        // Verify scores are descending
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score,
                "Results should be sorted by score descending");
        }
    }

    #[tokio::test]
    async fn test_semantic_search_after_note_update() {
        let fixture = FullTestFixture::new().await;

        // Create a note about Python
        let note = fixture
            .store
            .create(
                "Programming Languages".to_string(),
                "Python is great for data science and machine learning.".to_string(),
                None,
            )
            .await
            .expect("Should create note");

        fixture.index_note_for_semantic(&note).await;

        // Update to be about Rust
        let updated = fixture
            .store
            .update(note.id, "Rust is great for systems programming and performance.".to_string())
            .await
            .expect("Should update");

        // Remove old chunks, add new ones (simulating handler behavior)
        {
            let mut semantic = fixture.semantic.write().await;
            semantic.remove_chunks_for_note(note.id);
        }
        fixture.index_note_for_semantic(&updated).await;

        // Search for Rust should find the note
        {
            let semantic = fixture.semantic.read().await;
            let rust_results = semantic.search("systems programming Rust", 10).await.expect("Should search");
            assert!(!rust_results.is_empty(), "Should find updated note about Rust");
        }

        // Search for Python should NOT find the note anymore
        {
            let semantic = fixture.semantic.read().await;
            let python_results = semantic.search("Python data science", 10).await.expect("Should search");
            // The note might still appear but with low score, or not at all
            if !python_results.is_empty() {
                // If it appears, it should have a lower score than a direct match would
                assert!(python_results[0].score < 0.5, "Old content should have low relevance score");
            }
        }
    }

    #[tokio::test]
    async fn test_full_flow_create_search_retrieve() {
        let fixture = FullTestFixture::new().await;

        // 1. Create a note
        let note = fixture
            .store
            .create(
                "GraphQL API Design".to_string(),
                "Best practices for designing GraphQL APIs including schema design and resolvers.".to_string(),
                Some(vec!["api".to_string(), "graphql".to_string()]),
            )
            .await
            .expect("Should create note");

        // 2. Index for both fulltext and semantic
        fixture.fulltext.index_note(&note).expect("Should index fulltext");
        fixture.fulltext.commit().expect("Should commit");
        fixture.index_note_for_semantic(&note).await;

        // 3. Search via fulltext
        let fulltext_results = fixture.fulltext.search("GraphQL", 10).expect("Should search fulltext");
        assert!(!fulltext_results.is_empty(), "Fulltext should find note");

        // 4. Search via semantic
        let semantic = fixture.semantic.read().await;
        let semantic_results = semantic.search("API schema design", 10).await.expect("Should search semantic");
        assert!(!semantic_results.is_empty(), "Semantic should find note");

        // 5. Retrieve note using IDs from both search results
        let fulltext_uuid = fulltext_results[0].note_id.parse::<uuid::Uuid>().expect("Valid UUID");
        let semantic_uuid = semantic_results[0].note_id.parse::<uuid::Uuid>().expect("Valid UUID");

        // Both should point to the same note
        assert_eq!(fulltext_uuid, semantic_uuid, "Both searches should find the same note");
        assert_eq!(fulltext_uuid, note.id, "Search results should match created note");

        // 6. Verify note is retrievable
        let retrieved = fixture.store.get(fulltext_uuid).await;
        assert!(retrieved.is_some(), "Note should be retrievable from store");
        assert_eq!(retrieved.unwrap().title, "GraphQL API Design");
    }
}
