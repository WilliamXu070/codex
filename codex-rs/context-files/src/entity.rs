//! Entity extraction from text using pattern matching.
//!
//! Extracts named entities (people, projects, technologies, dates, etc.)
//! from document chunks using regex patterns and heuristics.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::chunker::Chunk;

/// An extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier.
    pub id: String,

    /// The entity name/value.
    pub name: String,

    /// Normalized form of the name (lowercase, trimmed).
    pub normalized_name: String,

    /// Type of entity.
    pub entity_type: EntityType,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// Chunks where this entity was found.
    pub mentions: Vec<EntityMention>,

    /// Additional attributes.
    pub attributes: HashMap<String, String>,
}

impl Entity {
    /// Create a new entity.
    pub fn new(name: impl Into<String>, entity_type: EntityType, confidence: f32) -> Self {
        let name = name.into();
        let normalized_name = Self::normalize(&name);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            normalized_name,
            entity_type,
            confidence,
            mentions: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Add a mention of this entity.
    pub fn add_mention(&mut self, mention: EntityMention) {
        self.mentions.push(mention);
    }

    /// Set an attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Normalize an entity name for comparison.
    fn normalize(name: &str) -> String {
        name.to_lowercase()
            .trim()
            .replace(['_', '-'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Check if two entities are likely the same.
    pub fn is_same_as(&self, other: &Entity) -> bool {
        self.entity_type == other.entity_type && self.normalized_name == other.normalized_name
    }

    /// Merge another entity into this one.
    pub fn merge(&mut self, other: Entity) {
        self.mentions.extend(other.mentions);
        for (k, v) in other.attributes {
            self.attributes.entry(k).or_insert(v);
        }
        // Keep the higher confidence
        self.confidence = self.confidence.max(other.confidence);
    }
}

/// Type of entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// A person (author, contributor, user).
    Person,
    /// A project or repository.
    Project,
    /// A technology, framework, or library.
    Technology,
    /// A date or time reference.
    Date,
    /// A location or path.
    Location,
    /// An organization or company.
    Organization,
    /// A version number.
    Version,
    /// A URL or link.
    Url,
    /// An email address.
    Email,
    /// A generic concept or topic.
    Concept,
    /// A file or directory reference.
    File,
    /// A function, class, or code element.
    CodeElement,
}

impl EntityType {
    /// Get a display name for this entity type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::Project => "Project",
            Self::Technology => "Technology",
            Self::Date => "Date",
            Self::Location => "Location",
            Self::Organization => "Organization",
            Self::Version => "Version",
            Self::Url => "URL",
            Self::Email => "Email",
            Self::Concept => "Concept",
            Self::File => "File",
            Self::CodeElement => "Code Element",
        }
    }
}

/// A mention of an entity in a chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    /// Chunk ID where the entity was found.
    pub chunk_id: String,

    /// Position in the chunk (character offset).
    pub position: usize,

    /// The exact text that matched.
    pub matched_text: String,

    /// Context around the mention.
    pub context: Option<String>,
}

/// Configuration for entity extraction.
#[derive(Debug, Clone)]
pub struct EntityExtractorConfig {
    /// Minimum confidence threshold.
    pub min_confidence: f32,

    /// Whether to extract people.
    pub extract_people: bool,

    /// Whether to extract projects.
    pub extract_projects: bool,

    /// Whether to extract technologies.
    pub extract_technologies: bool,

    /// Whether to extract dates.
    pub extract_dates: bool,

    /// Whether to extract URLs.
    pub extract_urls: bool,

    /// Whether to extract emails.
    pub extract_emails: bool,

    /// Whether to extract file references.
    pub extract_files: bool,

    /// Whether to extract code elements.
    pub extract_code_elements: bool,

    /// Context window size (chars before/after mention).
    pub context_window: usize,
}

impl Default for EntityExtractorConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            extract_people: true,
            extract_projects: true,
            extract_technologies: true,
            extract_dates: true,
            extract_urls: true,
            extract_emails: true,
            extract_files: true,
            extract_code_elements: true,
            context_window: 50,
        }
    }
}

/// Entity extractor using pattern matching.
pub struct EntityExtractor {
    config: EntityExtractorConfig,
    known_technologies: HashSet<String>,
}

impl EntityExtractor {
    /// Create a new entity extractor with default configuration.
    pub fn new() -> Self {
        Self {
            config: EntityExtractorConfig::default(),
            known_technologies: Self::default_technologies(),
        }
    }

    /// Create an extractor with custom configuration.
    pub fn with_config(config: EntityExtractorConfig) -> Self {
        Self {
            config,
            known_technologies: Self::default_technologies(),
        }
    }

    /// Get default known technologies.
    fn default_technologies() -> HashSet<String> {
        [
            // Languages
            "rust",
            "python",
            "javascript",
            "typescript",
            "java",
            "go",
            "golang",
            "c",
            "c++",
            "cpp",
            "csharp",
            "c#",
            "ruby",
            "swift",
            "kotlin",
            "scala",
            "php",
            "perl",
            "haskell",
            "elixir",
            "clojure",
            "lua",
            "r",
            "julia",
            // Frameworks
            "react",
            "vue",
            "angular",
            "svelte",
            "nextjs",
            "next.js",
            "nuxt",
            "django",
            "flask",
            "fastapi",
            "rails",
            "spring",
            "express",
            "nestjs",
            "actix",
            "axum",
            "rocket",
            "tokio",
            "async-std",
            // Tools
            "git",
            "docker",
            "kubernetes",
            "k8s",
            "terraform",
            "ansible",
            "jenkins",
            "github",
            "gitlab",
            "bitbucket",
            "npm",
            "yarn",
            "pnpm",
            "cargo",
            "pip",
            "maven",
            "gradle",
            "webpack",
            "vite",
            "esbuild",
            // Databases
            "postgresql",
            "postgres",
            "mysql",
            "mongodb",
            "redis",
            "elasticsearch",
            "sqlite",
            "dynamodb",
            "cassandra",
            "neo4j",
            "supabase",
            // Cloud
            "aws",
            "azure",
            "gcp",
            "google cloud",
            "heroku",
            "vercel",
            "netlify",
            "cloudflare",
            "digitalocean",
            // AI/ML
            "openai",
            "anthropic",
            "claude",
            "gpt",
            "llama",
            "pytorch",
            "tensorflow",
            "huggingface",
            "langchain",
            "llamaindex",
        ]
        .iter()
        .map(|s| s.to_lowercase())
        .collect()
    }

    /// Extract entities from a list of chunks.
    pub fn extract(&self, chunks: &[Chunk]) -> Vec<Entity> {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        for chunk in chunks {
            let chunk_entities = self.extract_from_chunk(chunk);

            for entity in chunk_entities {
                let key = format!("{:?}:{}", entity.entity_type, entity.normalized_name);

                entities
                    .entry(key)
                    .and_modify(|e| e.merge(entity.clone()))
                    .or_insert(entity);
            }
        }

        // Filter by confidence and return
        entities
            .into_values()
            .filter(|e| e.confidence >= self.config.min_confidence)
            .collect()
    }

    /// Extract entities from a single chunk.
    fn extract_from_chunk(&self, chunk: &Chunk) -> Vec<Entity> {
        let mut entities = Vec::new();
        let text = &chunk.content;

        // Extract different entity types
        if self.config.extract_people {
            entities.extend(self.extract_people(text, &chunk.id));
        }

        if self.config.extract_projects {
            entities.extend(self.extract_projects(text, &chunk.id));
        }

        if self.config.extract_technologies {
            entities.extend(self.extract_technologies(text, &chunk.id));
        }

        if self.config.extract_dates {
            entities.extend(self.extract_dates(text, &chunk.id));
        }

        if self.config.extract_urls {
            entities.extend(self.extract_urls(text, &chunk.id));
        }

        if self.config.extract_emails {
            entities.extend(self.extract_emails(text, &chunk.id));
        }

        if self.config.extract_files {
            entities.extend(self.extract_files(text, &chunk.id));
        }

        if self.config.extract_code_elements {
            entities.extend(self.extract_code_elements(text, &chunk.id));
        }

        entities
    }

    /// Extract people entities.
    fn extract_people(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Pattern: "by [Name]", "author: [Name]", "created by [Name]"
        let author_patterns = [
            r"(?i)(?:by|author|created by|maintained by|written by)\s*:?\s*([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)",
            r"@([a-zA-Z][\w-]+)", // GitHub mentions
            r"(?i)contributor[s]?:\s*([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)",
        ];

        for pattern in author_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    if let Some(name) = cap.get(1) {
                        let name_str = name.as_str().to_string();
                        if name_str.len() >= 2 && name_str.len() <= 50 {
                            let mut entity = Entity::new(&name_str, EntityType::Person, 0.8);
                            entity.add_mention(EntityMention {
                                chunk_id: chunk_id.to_string(),
                                position: name.start(),
                                matched_text: name_str.clone(),
                                context: self.get_context(text, name.start(), name.end()),
                            });
                            entities.push(entity);
                        }
                    }
                }
            }
        }

        entities
    }

    /// Extract project entities.
    fn extract_projects(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Pattern: "project: [name]", "[org]/[repo]"
        let project_patterns = [
            r"(?i)(?:project|repo|repository):\s*[`]?([a-zA-Z][\w-]+)[`]?",
            r"(?:github\.com|gitlab\.com)/([a-zA-Z][\w-]+/[a-zA-Z][\w-]+)",
            r#"name\s*[=:]\s*["']([a-zA-Z][\w-]+)["']"#, // package.json, Cargo.toml
        ];

        for pattern in project_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    if let Some(name) = cap.get(1) {
                        let name_str = name.as_str().to_string();
                        let mut entity = Entity::new(&name_str, EntityType::Project, 0.9);
                        entity.add_mention(EntityMention {
                            chunk_id: chunk_id.to_string(),
                            position: name.start(),
                            matched_text: name_str,
                            context: self.get_context(text, name.start(), name.end()),
                        });
                        entities.push(entity);
                    }
                }
            }
        }

        entities
    }

    /// Extract technology entities.
    fn extract_technologies(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Check for known technologies
        for tech in &self.known_technologies {
            // Word boundary matching
            let pattern = format!(r"\b{}\b", regex_lite::escape(tech));
            if let Ok(re) = regex_lite::Regex::new(&pattern) {
                for mat in re.find_iter(&text_lower) {
                    // Get the original case from the text
                    let original = &text[mat.start()..mat.end()];
                    let mut entity = Entity::new(original, EntityType::Technology, 0.9);
                    entity.add_mention(EntityMention {
                        chunk_id: chunk_id.to_string(),
                        position: mat.start(),
                        matched_text: original.to_string(),
                        context: self.get_context(text, mat.start(), mat.end()),
                    });
                    entities.push(entity);
                }
            }
        }

        // Pattern: "using [Tech]", "built with [Tech]"
        let tech_patterns = [
            r"(?i)(?:using|built with|powered by|requires|depends on)\s+([A-Z][a-zA-Z0-9]+(?:\s+[\d.]+)?)",
        ];

        for pattern in tech_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    if let Some(tech) = cap.get(1) {
                        let tech_str = tech.as_str().to_string();
                        if !self.known_technologies.contains(&tech_str.to_lowercase()) {
                            let mut entity = Entity::new(&tech_str, EntityType::Technology, 0.7);
                            entity.add_mention(EntityMention {
                                chunk_id: chunk_id.to_string(),
                                position: tech.start(),
                                matched_text: tech_str,
                                context: self.get_context(text, tech.start(), tech.end()),
                            });
                            entities.push(entity);
                        }
                    }
                }
            }
        }

        entities
    }

    /// Extract date entities.
    fn extract_dates(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let date_patterns = [
            r"\b(\d{4}-\d{2}-\d{2})\b",     // ISO date
            r"\b(\d{1,2}/\d{1,2}/\d{4})\b", // US date
            r"\b(\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\s+\d{4})\b",
            r"(?i)(?:deadline|due|by|on)\s+(\w+\s+\d+(?:,?\s+\d{4})?)",
        ];

        for pattern in date_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    if let Some(date) = cap.get(1) {
                        let date_str = date.as_str().to_string();
                        let mut entity = Entity::new(&date_str, EntityType::Date, 0.95);
                        entity.add_mention(EntityMention {
                            chunk_id: chunk_id.to_string(),
                            position: date.start(),
                            matched_text: date_str,
                            context: self.get_context(text, date.start(), date.end()),
                        });
                        entities.push(entity);
                    }
                }
            }
        }

        entities
    }

    /// Extract URL entities.
    fn extract_urls(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let url_pattern = r"https?://[^\s\)<>\]\[]+";
        if let Ok(re) = regex_lite::Regex::new(url_pattern) {
            for mat in re.find_iter(text) {
                let url = mat.as_str().trim_end_matches(&['.', ',', ')', ']'][..]);
                let mut entity = Entity::new(url, EntityType::Url, 1.0);
                entity.add_mention(EntityMention {
                    chunk_id: chunk_id.to_string(),
                    position: mat.start(),
                    matched_text: url.to_string(),
                    context: self.get_context(text, mat.start(), mat.end()),
                });
                entities.push(entity);
            }
        }

        entities
    }

    /// Extract email entities.
    fn extract_emails(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let email_pattern = r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}";
        if let Ok(re) = regex_lite::Regex::new(email_pattern) {
            for mat in re.find_iter(text) {
                let email = mat.as_str();
                let mut entity = Entity::new(email, EntityType::Email, 1.0);
                entity.add_mention(EntityMention {
                    chunk_id: chunk_id.to_string(),
                    position: mat.start(),
                    matched_text: email.to_string(),
                    context: self.get_context(text, mat.start(), mat.end()),
                });
                entities.push(entity);
            }
        }

        entities
    }

    /// Extract file path entities.
    fn extract_files(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let file_patterns = [
            r"`([a-zA-Z][\w./\-]+\.[a-zA-Z]+)`", // Markdown code: `path/file.ext`
            r"(?:src|lib|bin|tests?)/[\w./\-]+\.[a-zA-Z]+", // Common source paths
        ];

        for pattern in file_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    let file = cap
                        .get(1)
                        .map(|m| m.as_str())
                        .unwrap_or(cap.get(0).unwrap().as_str());
                    if file.len() >= 3 && file.len() <= 100 {
                        let mut entity = Entity::new(file, EntityType::File, 0.85);
                        entity.add_mention(EntityMention {
                            chunk_id: chunk_id.to_string(),
                            position: cap.get(0).unwrap().start(),
                            matched_text: file.to_string(),
                            context: None,
                        });
                        entities.push(entity);
                    }
                }
            }
        }

        entities
    }

    /// Extract code element entities (functions, classes, etc.).
    fn extract_code_elements(&self, text: &str, chunk_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let code_patterns = [
            r"(?:fn|func|function|def)\s+([a-zA-Z_][a-zA-Z0-9_]*)", // Function definitions
            r"(?:struct|class|type|interface)\s+([A-Z][a-zA-Z0-9_]*)", // Type definitions
            r"(?:const|let|var)\s+([A-Z_][A-Z0-9_]*)\s*=",          // Constants
        ];

        for pattern in code_patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                for cap in re.captures_iter(text) {
                    if let Some(name) = cap.get(1) {
                        let name_str = name.as_str();
                        if name_str.len() >= 2 {
                            let mut entity = Entity::new(name_str, EntityType::CodeElement, 0.9);
                            entity.add_mention(EntityMention {
                                chunk_id: chunk_id.to_string(),
                                position: name.start(),
                                matched_text: name_str.to_string(),
                                context: self.get_context(text, name.start(), name.end()),
                            });
                            entities.push(entity);
                        }
                    }
                }
            }
        }

        entities
    }

    /// Get context around a mention.
    fn get_context(&self, text: &str, start: usize, end: usize) -> Option<String> {
        let window = self.config.context_window;
        let ctx_start = start.saturating_sub(window);
        let ctx_end = (end + window).min(text.len());

        let context = &text[ctx_start..ctx_end];
        if context.len() > 10 {
            Some(context.to_string())
        } else {
            None
        }
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn make_chunk(content: &str) -> Chunk {
        Chunk::new(content.to_string(), crate::chunker::ChunkType::Text)
    }

    #[test]
    fn test_extract_person() {
        let extractor = EntityExtractor::new();
        let chunks = vec![make_chunk("This project was created by John Smith.")];

        let entities = extractor.extract(&chunks);
        let people: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Person)
            .collect();

        assert!(!people.is_empty());
        assert!(people.iter().any(|e| e.name.contains("John")));
    }

    #[test]
    fn test_extract_technology() {
        let extractor = EntityExtractor::new();
        let chunks = vec![make_chunk(
            "Built with Rust and TypeScript. Uses Docker for deployment.",
        )];

        let entities = extractor.extract(&chunks);
        let techs: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Technology)
            .collect();

        assert!(techs.len() >= 2);
        let tech_names: Vec<_> = techs.iter().map(|t| t.normalized_name.as_str()).collect();
        assert!(tech_names.contains(&"rust"));
        assert!(tech_names.contains(&"typescript"));
    }

    #[test]
    fn test_extract_date() {
        let extractor = EntityExtractor::new();
        let chunks = vec![make_chunk("Deadline: 2024-12-31. Due by January 15, 2025.")];

        let entities = extractor.extract(&chunks);
        let dates: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Date)
            .collect();

        assert!(!dates.is_empty());
    }

    #[test]
    fn test_extract_url() {
        let extractor = EntityExtractor::new();
        let chunks = vec![make_chunk(
            "See https://github.com/user/repo for more info.",
        )];

        let entities = extractor.extract(&chunks);
        let urls: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Url)
            .collect();

        assert_eq!(urls.len(), 1);
        assert!(urls[0].name.contains("github.com"));
    }

    #[test]
    fn test_extract_email() {
        let extractor = EntityExtractor::new();
        let chunks = vec![make_chunk("Contact: john@example.com")];

        let entities = extractor.extract(&chunks);
        let emails: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Email)
            .collect();

        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].name, "john@example.com");
    }

    #[test]
    fn test_entity_merging() {
        let extractor = EntityExtractor::new();
        let chunks = vec![
            make_chunk("Using Rust for performance."),
            make_chunk("Rust provides memory safety."),
        ];

        let entities = extractor.extract(&chunks);
        let rust_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.normalized_name == "rust")
            .collect();

        // Should be merged into one entity with multiple mentions
        assert_eq!(rust_entities.len(), 1);
        assert!(rust_entities[0].mentions.len() >= 2);
    }
}
