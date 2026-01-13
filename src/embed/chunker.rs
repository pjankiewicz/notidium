//! Content chunking for embeddings

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use uuid::Uuid;

use crate::types::{Chunk, ChunkType, Note};

/// Chunker for splitting notes into embeddable chunks
pub struct Chunker {
    /// Target words per chunk
    target_words: usize,
}

impl Default for Chunker {
    fn default() -> Self {
        Self { target_words: 250 }
    }
}

impl Chunker {
    pub fn new(target_words: usize) -> Self {
        Self { target_words }
    }

    /// Chunk a note into embeddable pieces
    pub fn chunk_note(&self, note: &Note) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let parser = Parser::new(&note.content);

        let mut current_text = String::new();
        let mut current_type = ChunkType::Prose;
        let mut in_code_block = false;
        let mut code_language = String::new();
        let mut line_number = 1u32;
        let mut chunk_start_line = 1u32;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Flush current chunk
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            current_type.clone(),
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }
                    current_type = ChunkType::Heading {
                        level: level as u8,
                    };
                    chunk_start_line = line_number;
                }
                Event::End(TagEnd::Heading(_)) => {
                    // Heading is its own chunk
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            current_type.clone(),
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }
                    current_type = ChunkType::Prose;
                    chunk_start_line = line_number;
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    // Flush current chunk
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            current_type.clone(),
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }

                    in_code_block = true;
                    code_language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    chunk_start_line = line_number;
                }
                Event::End(TagEnd::CodeBlock) => {
                    // Code block is its own chunk
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            ChunkType::CodeBlock {
                                language: code_language.clone(),
                                title: None,
                            },
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }

                    in_code_block = false;
                    code_language.clear();
                    current_type = ChunkType::Prose;
                    chunk_start_line = line_number;
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            current_type.clone(),
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }
                    current_type = ChunkType::Blockquote;
                    chunk_start_line = line_number;
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    if !current_text.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            note.id,
                            &current_text,
                            current_type.clone(),
                            chunk_start_line,
                            line_number,
                        ));
                        current_text.clear();
                    }
                    current_type = ChunkType::Prose;
                    chunk_start_line = line_number;
                }
                Event::Text(text) | Event::Code(text) => {
                    current_text.push_str(&text);
                    line_number += text.matches('\n').count() as u32;

                    // Check if we should split (for prose only)
                    if !in_code_block && !matches!(current_type, ChunkType::Heading { .. }) {
                        let word_count = current_text.split_whitespace().count();
                        if word_count >= self.target_words {
                            chunks.push(self.create_chunk(
                                note.id,
                                &current_text,
                                current_type.clone(),
                                chunk_start_line,
                                line_number,
                            ));
                            current_text.clear();
                            chunk_start_line = line_number;
                        }
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_text.push('\n');
                    line_number += 1;
                }
                _ => {}
            }
        }

        // Flush remaining content
        if !current_text.trim().is_empty() {
            chunks.push(self.create_chunk(
                note.id,
                &current_text,
                current_type,
                chunk_start_line,
                line_number,
            ));
        }

        chunks
    }

    fn create_chunk(
        &self,
        note_id: Uuid,
        content: &str,
        chunk_type: ChunkType,
        start_line: u32,
        end_line: u32,
    ) -> Chunk {
        let language = match &chunk_type {
            ChunkType::CodeBlock { language, .. } if !language.is_empty() => {
                Some(language.clone())
            }
            _ => None,
        };

        Chunk {
            id: Uuid::new_v4(),
            note_id,
            content: content.trim().to_string(),
            chunk_type,
            language,
            start_line,
            end_line,
            start_offset: 0,
            end_offset: 0,
            prose_embedding: None,
            code_embedding: None,
            embedding_model: None,
            embedded_at: None,
        }
    }
}
