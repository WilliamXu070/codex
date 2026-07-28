//! Semantic document chunking for context extraction.
//!
//! This module implements recursive chunking with semantic boundaries,
//! optimized for RAG-style retrieval. Chunks are split on natural boundaries
//! (paragraphs, sections) rather than fixed character counts.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A chunk of text extracted from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique identifier for this chunk.
    pub id: String,

    /// The actual text content.
    pub content: String,

    /// Source file path (if from a file).
    pub source: Option<String>,

    /// Type of content in this chunk.
    pub chunk_type: ChunkType,

    /// Position in the original document (character offset).
    pub start_offset: usize,

    /// End position in the original document.
    pub end_offset: usize,

    /// Parent chunk ID (for hierarchical chunking).
    pub parent_id: Option<String>,

    /// Metadata about the chunk.
    pub metadata: ChunkMetadata,
}

impl Chunk {
    /// Create a new chunk.
    pub fn new(content: impl Into<String>, chunk_type: ChunkType) -> Self {
        let content = content.into();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            source: None,
            chunk_type,
            start_offset: 0,
            end_offset: 0,
            parent_id: None,
            metadata: ChunkMetadata::default(),
        }
    }

    /// Set the source file.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the offsets.
    pub fn with_offsets(mut self, start: usize, end: usize) -> Self {
        self.start_offset = start;
        self.end_offset = end;
        self
    }

    /// Set the parent chunk.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Estimate token count (rough approximation: ~4 chars per token).
    pub fn estimated_tokens(&self) -> usize {
        self.content.len() / 4
    }

    /// Check if chunk is within token limit.
    pub fn within_limit(&self, max_tokens: usize) -> bool {
        self.estimated_tokens() <= max_tokens
    }
}

/// Type of chunk content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// Full document.
    Document,
    /// A section with a heading.
    Section,
    /// A paragraph of text.
    Paragraph,
    /// A code block.
    Code,
    /// A list (bullet points, numbered).
    List,
    /// A table.
    Table,
    /// Frontmatter (YAML, TOML).
    Frontmatter,
    /// Generic text chunk.
    Text,
}

/// Metadata about a chunk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Heading level (1-6 for markdown headers).
    pub heading_level: Option<u8>,

    /// Section title if this is a section.
    pub title: Option<String>,

    /// Programming language if this is code.
    pub language: Option<String>,

    /// Line number in original file.
    pub line_number: Option<usize>,

    /// Whether this chunk continues from previous.
    pub is_continuation: bool,
}

/// Configuration for the chunker.
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Target chunk size in tokens.
    pub target_tokens: usize,

    /// Maximum chunk size in tokens.
    pub max_tokens: usize,

    /// Overlap between chunks (as fraction, e.g., 0.2 = 20%).
    pub overlap_fraction: f32,

    /// Minimum chunk size in tokens.
    pub min_tokens: usize,

    /// Whether to preserve code blocks intact.
    pub preserve_code_blocks: bool,

    /// Whether to include section headers in chunks.
    pub include_headers: bool,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            target_tokens: 512,
            max_tokens: 1024,
            overlap_fraction: 0.2,
            min_tokens: 50,
            preserve_code_blocks: true,
            include_headers: true,
        }
    }
}

/// Semantic document chunker.
///
/// Splits documents into chunks based on semantic boundaries:
/// 1. Section headers (markdown #, ##, etc.)
/// 2. Paragraph breaks (double newlines)
/// 3. Code block boundaries
/// 4. List boundaries
pub struct SemanticChunker {
    config: ChunkerConfig,
}

impl SemanticChunker {
    /// Create a new chunker with default configuration.
    pub fn new() -> Self {
        Self {
            config: ChunkerConfig::default(),
        }
    }

    /// Create a chunker with custom configuration.
    pub fn with_config(config: ChunkerConfig) -> Self {
        Self { config }
    }

    /// Chunk a document from a file path.
    pub fn chunk_file(&self, path: &Path) -> std::io::Result<Vec<Chunk>> {
        let content = std::fs::read_to_string(path)?;
        let source = path.to_string_lossy().to_string();
        Ok(self.chunk_with_source(&content, &source))
    }

    /// Chunk text content with a source identifier.
    pub fn chunk_with_source(&self, content: &str, source: &str) -> Vec<Chunk> {
        let mut chunks = self.chunk(content);
        for chunk in &mut chunks {
            chunk.source = Some(source.to_string());
        }
        chunks
    }

    /// Chunk text content.
    pub fn chunk(&self, content: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        // First, identify structural elements
        let elements = self.parse_structure(content);

        // Then, chunk each element appropriately
        for element in elements {
            let element_chunks = self.chunk_element(&element);
            chunks.extend(element_chunks);
        }

        // Apply overlap if configured
        if self.config.overlap_fraction > 0.0 {
            chunks = self.apply_overlap(chunks);
        }

        chunks
    }

    /// Parse document structure into elements.
    fn parse_structure(&self, content: &str) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let mut current_offset = 0;
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let line_start = current_offset;

            // Check for markdown header
            if let Some(level) = self.detect_header_level(line) {
                let title = line.trim_start_matches('#').trim().to_string();
                elements.push(StructuralElement {
                    content: line.to_string(),
                    element_type: ChunkType::Section,
                    start_offset: line_start,
                    end_offset: line_start + line.len(),
                    metadata: ChunkMetadata {
                        heading_level: Some(level),
                        title: Some(title),
                        ..Default::default()
                    },
                });
                current_offset += line.len() + 1;
                i += 1;
                continue;
            }

            // Check for code block
            if line.trim().starts_with("```") {
                let (code_block, lines_consumed) = self.extract_code_block(&lines[i..]);
                let end_offset = current_offset + code_block.len();

                let language = line
                    .trim()
                    .strip_prefix("```")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                elements.push(StructuralElement {
                    content: code_block,
                    element_type: ChunkType::Code,
                    start_offset: current_offset,
                    end_offset,
                    metadata: ChunkMetadata {
                        language,
                        line_number: Some(i + 1),
                        ..Default::default()
                    },
                });

                for _ in 0..lines_consumed {
                    if i < lines.len() {
                        current_offset += lines[i].len() + 1;
                        i += 1;
                    }
                }
                continue;
            }

            // Check for list
            if self.is_list_item(line) {
                let (list_content, lines_consumed) = self.extract_list(&lines[i..]);
                let end_offset = current_offset + list_content.len();

                elements.push(StructuralElement {
                    content: list_content,
                    element_type: ChunkType::List,
                    start_offset: current_offset,
                    end_offset,
                    metadata: ChunkMetadata {
                        line_number: Some(i + 1),
                        ..Default::default()
                    },
                });

                for _ in 0..lines_consumed {
                    if i < lines.len() {
                        current_offset += lines[i].len() + 1;
                        i += 1;
                    }
                }
                continue;
            }

            // Regular paragraph - collect until empty line or structural element
            let (paragraph, lines_consumed) = self.extract_paragraph(&lines[i..]);
            if !paragraph.trim().is_empty() {
                let end_offset = current_offset + paragraph.len();
                elements.push(StructuralElement {
                    content: paragraph,
                    element_type: ChunkType::Paragraph,
                    start_offset: current_offset,
                    end_offset,
                    metadata: ChunkMetadata {
                        line_number: Some(i + 1),
                        ..Default::default()
                    },
                });
            }

            for _ in 0..lines_consumed {
                if i < lines.len() {
                    current_offset += lines[i].len() + 1;
                    i += 1;
                }
            }
        }

        elements
    }

    /// Detect markdown header level (1-6).
    fn detect_header_level(&self, line: &str) -> Option<u8> {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return None;
        }

        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if level > 0 && level <= 6 {
            // Ensure there's a space after the #'s
            let rest = &trimmed[level..];
            if rest.starts_with(' ') || rest.is_empty() {
                return Some(level as u8);
            }
        }
        None
    }

    /// Check if a line is a list item.
    fn is_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) && trimmed.contains(". ")
    }

    /// Extract a code block starting at the current position.
    fn extract_code_block(&self, lines: &[&str]) -> (String, usize) {
        let mut content = String::new();
        let mut count = 0;
        let mut in_block = false;

        for line in lines {
            content.push_str(line);
            content.push('\n');
            count += 1;

            if line.trim().starts_with("```") {
                if in_block {
                    break; // End of code block
                }
                in_block = true;
            }
        }

        (content.trim_end().to_string(), count)
    }

    /// Extract a list starting at the current position.
    fn extract_list(&self, lines: &[&str]) -> (String, usize) {
        let mut content = String::new();
        let mut count = 0;

        for line in lines {
            if !self.is_list_item(line) && !line.trim().is_empty() && !line.starts_with("  ") {
                break;
            }
            content.push_str(line);
            content.push('\n');
            count += 1;

            if line.trim().is_empty() && count > 1 {
                break;
            }
        }

        (content.trim_end().to_string(), count.max(1))
    }

    /// Extract a paragraph (until empty line or structural element).
    fn extract_paragraph(&self, lines: &[&str]) -> (String, usize) {
        let mut content = String::new();
        let mut count = 0;

        for line in lines {
            // Stop at empty lines
            if line.trim().is_empty() {
                count += 1;
                break;
            }

            // Stop at structural elements
            if self.detect_header_level(line).is_some()
                || line.trim().starts_with("```")
                || self.is_list_item(line)
            {
                break;
            }

            content.push_str(line);
            content.push('\n');
            count += 1;
        }

        (content.trim_end().to_string(), count.max(1))
    }

    /// Chunk a structural element into appropriately sized chunks.
    fn chunk_element(&self, element: &StructuralElement) -> Vec<Chunk> {
        let estimated_tokens = element.content.len() / 4;

        // If element fits in target size, return as single chunk
        if estimated_tokens <= self.config.max_tokens {
            return vec![Chunk {
                id: uuid::Uuid::new_v4().to_string(),
                content: element.content.clone(),
                source: None,
                chunk_type: element.element_type,
                start_offset: element.start_offset,
                end_offset: element.end_offset,
                parent_id: None,
                metadata: element.metadata.clone(),
            }];
        }

        // Need to split - use recursive character splitting
        self.recursive_split(element)
    }

    /// Recursively split large elements.
    fn recursive_split(&self, element: &StructuralElement) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let separators = ["\n\n", "\n", ". ", " "];

        self.split_recursive(
            &element.content,
            &separators,
            0,
            element.start_offset,
            element.element_type,
            &element.metadata,
            &mut chunks,
        );

        chunks
    }

    /// Recursive splitting helper.
    fn split_recursive(
        &self,
        text: &str,
        separators: &[&str],
        sep_index: usize,
        base_offset: usize,
        chunk_type: ChunkType,
        metadata: &ChunkMetadata,
        chunks: &mut Vec<Chunk>,
    ) {
        let estimated_tokens = text.len() / 4;

        // If small enough, add as chunk
        if estimated_tokens <= self.config.max_tokens || sep_index >= separators.len() {
            if estimated_tokens >= self.config.min_tokens {
                chunks.push(Chunk {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: text.to_string(),
                    source: None,
                    chunk_type,
                    start_offset: base_offset,
                    end_offset: base_offset + text.len(),
                    parent_id: None,
                    metadata: metadata.clone(),
                });
            }
            return;
        }

        // Split by current separator
        let separator = separators[sep_index];
        let parts: Vec<&str> = text.split(separator).collect();

        if parts.len() == 1 {
            // Separator not found, try next
            self.split_recursive(
                text,
                separators,
                sep_index + 1,
                base_offset,
                chunk_type,
                metadata,
                chunks,
            );
            return;
        }

        // Combine parts into chunks that fit
        let mut current_chunk = String::new();
        let mut current_offset = base_offset;

        for (i, part) in parts.iter().enumerate() {
            let test_chunk = if current_chunk.is_empty() {
                part.to_string()
            } else {
                format!("{}{}{}", current_chunk, separator, part)
            };

            let test_tokens = test_chunk.len() / 4;

            if test_tokens > self.config.max_tokens && !current_chunk.is_empty() {
                // Current chunk is full, recurse on it
                self.split_recursive(
                    &current_chunk,
                    separators,
                    sep_index + 1,
                    current_offset,
                    chunk_type,
                    metadata,
                    chunks,
                );
                current_chunk = part.to_string();
                current_offset = base_offset + text[..(text.find(part).unwrap_or(0))].len();
            } else {
                current_chunk = test_chunk;
            }

            // Add separator length to offset calculation
            if i < parts.len() - 1 {
                // Account for separator
            }
        }

        // Handle remaining content
        if !current_chunk.is_empty() {
            self.split_recursive(
                &current_chunk,
                separators,
                sep_index + 1,
                current_offset,
                chunk_type,
                metadata,
                chunks,
            );
        }
    }

    /// Apply overlap between chunks.
    fn apply_overlap(&self, chunks: Vec<Chunk>) -> Vec<Chunk> {
        if chunks.len() <= 1 {
            return chunks;
        }

        let overlap_chars =
            (self.config.target_tokens as f32 * 4.0 * self.config.overlap_fraction) as usize;

        let mut result: Vec<Chunk> = Vec::with_capacity(chunks.len());

        for (i, mut chunk) in chunks.into_iter().enumerate() {
            if i > 0 && overlap_chars > 0 {
                // Get overlap from previous chunk
                if let Some(prev) = result.last() {
                    if prev.content.len() > overlap_chars {
                        let overlap = &prev.content[prev.content.len() - overlap_chars..];
                        chunk.content = format!("{}{}", overlap, chunk.content);
                        chunk.metadata.is_continuation = true;
                    }
                }
            }
            result.push(chunk);
        }

        result
    }
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal structural element representation.
struct StructuralElement {
    content: String,
    element_type: ChunkType,
    start_offset: usize,
    end_offset: usize,
    metadata: ChunkMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_chunk_simple_text() {
        let chunker = SemanticChunker::new();
        let text = "This is a simple paragraph.\n\nThis is another paragraph.";

        let chunks = chunker.chunk(text);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_type, ChunkType::Paragraph);
    }

    #[test]
    fn test_chunk_with_headers() {
        let chunker = SemanticChunker::new();
        let text = "# Main Title\n\nSome content here.\n\n## Subtitle\n\nMore content.";

        let chunks = chunker.chunk(text);
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Section));
    }

    #[test]
    fn test_chunk_code_block() {
        let chunker = SemanticChunker::new();
        let text =
            "Some text.\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n\nMore text.";

        let chunks = chunker.chunk(text);
        let code_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::Code)
            .collect();
        assert_eq!(code_chunks.len(), 1);
        assert_eq!(code_chunks[0].metadata.language, Some("rust".to_string()));
    }

    #[test]
    fn test_chunk_list() {
        let chunker = SemanticChunker::new();
        let text = "Items:\n\n- First item\n- Second item\n- Third item\n\nEnd.";

        let chunks = chunker.chunk(text);
        let list_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.chunk_type == ChunkType::List)
            .collect();
        assert_eq!(list_chunks.len(), 1);
    }

    #[test]
    fn test_estimated_tokens() {
        let chunk = Chunk::new("This is a test with about 40 characters.", ChunkType::Text);
        assert_eq!(chunk.estimated_tokens(), 10); // ~40 chars / 4
    }

    #[test]
    fn test_header_detection() {
        let chunker = SemanticChunker::new();
        assert_eq!(chunker.detect_header_level("# Title"), Some(1));
        assert_eq!(chunker.detect_header_level("## Subtitle"), Some(2));
        assert_eq!(chunker.detect_header_level("### Deep"), Some(3));
        assert_eq!(chunker.detect_header_level("Not a header"), None);
        assert_eq!(chunker.detect_header_level("#hashtag"), None);
    }
}
