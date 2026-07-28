//! LLM integration layer for agentic context analysis.
//!
//! The `LlmAnalyzer` provides AI-powered document analysis, domain detection,
//! and cross-linking capabilities. It supports both LLM-based analysis and
//! heuristic fallbacks when the LLM is unavailable.

use std::collections::HashMap;

use tracing::info;

use crate::chunker::SemanticChunker;
use crate::entity::{Entity, EntityExtractor, EntityType};
use crate::error::Result;
use crate::node::{ContextNode, CrossLinkType, DocumentAnalysis, DomainDetection, RelatedNode};

/// Configuration for the LLM analyzer.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Whether to fall back to heuristics when LLM is unavailable.
    pub fallback_to_heuristic: bool,

    /// Minimum confidence to use LLM results.
    pub min_confidence: f32,

    /// Maximum tokens for document analysis.
    pub max_analysis_tokens: usize,

    /// Known domains for detection.
    pub known_domains: Vec<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            fallback_to_heuristic: true,
            min_confidence: 0.5,
            max_analysis_tokens: 4096,
            known_domains: vec![
                "coding".to_string(),
                "cooking".to_string(),
                "work".to_string(),
                "personal".to_string(),
                "education".to_string(),
                "finance".to_string(),
                "health".to_string(),
                "travel".to_string(),
            ],
        }
    }
}

/// Context for document analysis.
#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    /// The file path being analyzed.
    pub file_path: Option<String>,

    /// The file extension.
    pub file_extension: Option<String>,

    /// The parent folder name.
    pub parent_folder: Option<String>,

    /// Existing domains in the tree.
    pub existing_domains: Vec<String>,

    /// Hint about expected content type.
    pub content_hint: Option<String>,
}

/// LLM-powered analyzer for document analysis and domain detection.
///
/// The analyzer can operate in two modes:
/// 1. LLM mode: Uses AI for deep analysis (requires API connection)
/// 2. Heuristic mode: Uses pattern matching and rules (always available)
pub struct LlmAnalyzer {
    config: LlmConfig,
    entity_extractor: EntityExtractor,
    chunker: SemanticChunker,
    // TODO: Add ModelClient when integrating with codex-core
    // client: Option<ModelClient>,
}

impl Default for LlmAnalyzer {
    fn default() -> Self {
        Self::new(LlmConfig::default())
    }
}

impl LlmAnalyzer {
    /// Create a new LLM analyzer with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            entity_extractor: EntityExtractor::new(),
            chunker: SemanticChunker::new(),
        }
    }

    /// Create an analyzer with heuristic-only mode.
    pub fn heuristic_only() -> Self {
        let mut config = LlmConfig::default();
        config.fallback_to_heuristic = true;
        Self::new(config)
    }

    /// Check if LLM is available.
    pub fn is_llm_available(&self) -> bool {
        // TODO: Check if ModelClient is connected
        false
    }

    /// Analyze a document and extract structured information.
    pub async fn analyze_document(
        &self,
        content: &str,
        context: &AnalysisContext,
    ) -> Result<DocumentAnalysis> {
        if self.is_llm_available() {
            self.analyze_with_llm(content, context).await
        } else if self.config.fallback_to_heuristic {
            Ok(self.analyze_with_heuristics(content, context))
        } else {
            Ok(DocumentAnalysis::default())
        }
    }

    /// Analyze document using LLM.
    async fn analyze_with_llm(
        &self,
        content: &str,
        context: &AnalysisContext,
    ) -> Result<DocumentAnalysis> {
        // TODO: Implement LLM-based analysis using ModelClient
        // For now, fall back to heuristics
        info!("LLM analysis not yet implemented, using heuristics");
        Ok(self.analyze_with_heuristics(content, context))
    }

    /// Analyze document using heuristic methods.
    fn analyze_with_heuristics(
        &self,
        content: &str,
        context: &AnalysisContext,
    ) -> DocumentAnalysis {
        // Chunk the content first
        let chunks = self.chunker.chunk(content);

        // Extract entities using pattern matching
        let entities = self.entity_extractor.extract(&chunks);

        // Generate summary from first paragraph
        let summary = self.generate_heuristic_summary(content, &context.file_path);

        // Extract topics from entities and content
        let topics = self.extract_topics(&entities, content);

        // Detect domain from content and context
        let suggested_domain = if let Some(ref ext) = context.file_extension {
            let extensions = vec![ext.clone()];
            let detection = self.detect_domain_heuristic_full(content, &extensions, &[]);
            if detection.confidence > 0.5 {
                Some(detection.domain)
            } else {
                self.detect_domain_heuristic(content, context)
            }
        } else {
            self.detect_domain_heuristic(content, context)
        };

        // Calculate confidence based on available information
        let confidence = self.calculate_confidence(&entities, &topics, &suggested_domain);

        DocumentAnalysis {
            summary,
            entities,
            topics,
            suggested_domain,
            confidence,
        }
    }

    /// Generate a summary using heuristics.
    fn generate_heuristic_summary(&self, content: &str, file_path: &Option<String>) -> String {
        // Get first meaningful paragraph
        let lines: Vec<&str> = content.lines().collect();

        // Skip empty lines and headers
        let mut summary_lines = Vec::new();
        let mut in_content = false;

        for line in lines.iter().take(10) {
            let trimmed = line.trim();

            // Skip empty lines at start
            if trimmed.is_empty() && !in_content {
                continue;
            }

            // Skip markdown headers
            if trimmed.starts_with('#') {
                in_content = true;
                continue;
            }

            // Skip code blocks
            if trimmed.starts_with("```") {
                continue;
            }

            if !trimmed.is_empty() {
                in_content = true;
                summary_lines.push(trimmed);

                // Stop after getting enough content
                if summary_lines.join(" ").len() > 200 {
                    break;
                }
            }
        }

        let mut summary = summary_lines.join(" ");

        // Truncate if too long
        if summary.len() > 300 {
            summary = summary[..300].to_string();
            if let Some(last_space) = summary.rfind(' ') {
                summary = summary[..last_space].to_string();
            }
            summary.push_str("...");
        }

        // Add file context if summary is too short
        if summary.len() < 50 {
            if let Some(path) = file_path {
                if let Some(filename) = path.split(['/', '\\']).last() {
                    summary = format!("Content from {}: {}", filename, summary);
                }
            }
        }

        if summary.is_empty() {
            summary = "No summary available".to_string();
        }

        summary
    }

    /// Extract topics from entities and content.
    fn extract_topics(&self, entities: &[Entity], content: &str) -> Vec<String> {
        let mut topics = Vec::new();

        // Add technology entities as topics
        for entity in entities {
            if entity.entity_type == EntityType::Technology {
                let topic = entity.name.to_lowercase();
                if !topics.contains(&topic) {
                    topics.push(topic);
                }
            }
        }

        // Detect topics from content patterns
        let content_lower = content.to_lowercase();

        // Programming topics
        if content_lower.contains("async") || content_lower.contains("await") {
            topics.push("async-programming".to_string());
        }
        if content_lower.contains("test") || content_lower.contains("spec") {
            topics.push("testing".to_string());
        }
        if content_lower.contains("api") || content_lower.contains("endpoint") {
            topics.push("api".to_string());
        }
        if content_lower.contains("database") || content_lower.contains("sql") {
            topics.push("database".to_string());
        }
        if content_lower.contains("frontend") || content_lower.contains("ui") {
            topics.push("frontend".to_string());
        }
        if content_lower.contains("backend") || content_lower.contains("server") {
            topics.push("backend".to_string());
        }

        // Non-coding topics
        if content_lower.contains("recipe") || content_lower.contains("ingredient") {
            topics.push("recipe".to_string());
        }
        if content_lower.contains("meeting") || content_lower.contains("agenda") {
            topics.push("meeting".to_string());
        }
        if content_lower.contains("budget") || content_lower.contains("expense") {
            topics.push("finance".to_string());
        }

        topics
    }

    /// Detect domain using heuristics.
    fn detect_domain_heuristic(&self, content: &str, context: &AnalysisContext) -> Option<String> {
        let content_lower = content.to_lowercase();

        // Check file extension for coding
        if let Some(ref ext) = context.file_extension {
            let coding_extensions = [
                "rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "rb", "php", "swift", "kt",
                "scala", "toml", "yaml", "json", "xml", "html", "css",
            ];
            if coding_extensions.contains(&ext.as_str()) {
                return Some("coding".to_string());
            }
        }

        // Check content patterns
        let domain_patterns: Vec<(&str, &[&str])> = vec![
            (
                "coding",
                &[
                    "function", "class", "import", "export", "const", "let", "var", "def ", "fn ",
                    "pub ", "struct", "impl", "async", "await", "return",
                ],
            ),
            (
                "cooking",
                &[
                    "recipe",
                    "ingredient",
                    "cook",
                    "bake",
                    "fry",
                    "boil",
                    "tablespoon",
                    "teaspoon",
                    "cup",
                    "oven",
                    "preheat",
                    "serve",
                ],
            ),
            (
                "work",
                &[
                    "meeting",
                    "deadline",
                    "project",
                    "milestone",
                    "stakeholder",
                    "quarterly",
                    "budget",
                    "team",
                    "manager",
                    "report",
                ],
            ),
            (
                "personal",
                &[
                    "journal",
                    "diary",
                    "thoughts",
                    "feeling",
                    "today",
                    "yesterday",
                    "weekend",
                    "vacation",
                ],
            ),
            (
                "education",
                &[
                    "learn",
                    "study",
                    "course",
                    "lecture",
                    "homework",
                    "assignment",
                    "exam",
                    "grade",
                    "student",
                ],
            ),
            (
                "finance",
                &[
                    "budget",
                    "expense",
                    "income",
                    "investment",
                    "savings",
                    "tax",
                    "balance",
                    "account",
                    "bank",
                ],
            ),
        ];

        let mut scores: HashMap<&str, usize> = HashMap::new();

        for (domain, patterns) in &domain_patterns {
            let count = patterns
                .iter()
                .filter(|p| content_lower.contains(*p))
                .count();
            if count > 0 {
                scores.insert(domain, count);
            }
        }

        // Return domain with highest score if it meets threshold
        scores
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .filter(|(_, count)| *count >= 2)
            .map(|(domain, _)| domain.to_string())
    }

    /// Calculate confidence score.
    fn calculate_confidence(
        &self,
        entities: &[Entity],
        topics: &[String],
        domain: &Option<String>,
    ) -> f32 {
        let mut confidence = 0.3; // Base confidence

        // More entities = higher confidence
        if !entities.is_empty() {
            confidence += 0.1 * (entities.len() as f32).min(3.0) / 3.0;
        }

        // More topics = higher confidence
        if !topics.is_empty() {
            confidence += 0.1 * (topics.len() as f32).min(5.0) / 5.0;
        }

        // Domain detection = higher confidence
        if domain.is_some() {
            confidence += 0.2;
        }

        confidence.min(1.0)
    }

    /// Detect domain for a folder based on its contents.
    pub async fn detect_domain(
        &self,
        folder_summary: &str,
        file_extensions: &[String],
        existing_domains: &[String],
    ) -> Result<DomainDetection> {
        if self.is_llm_available() {
            self.detect_domain_with_llm(folder_summary, existing_domains)
                .await
        } else if self.config.fallback_to_heuristic {
            Ok(
                self.detect_domain_heuristic_full(
                    folder_summary,
                    file_extensions,
                    existing_domains,
                ),
            )
        } else {
            Ok(DomainDetection::new("other", 0.3).as_new())
        }
    }

    /// Detect domain using LLM.
    async fn detect_domain_with_llm(
        &self,
        _folder_summary: &str,
        _existing_domains: &[String],
    ) -> Result<DomainDetection> {
        // TODO: Implement LLM-based domain detection
        info!("LLM domain detection not yet implemented");
        Ok(DomainDetection::new("other", 0.3).as_new())
    }

    /// Detect domain using full heuristics.
    fn detect_domain_heuristic_full(
        &self,
        folder_summary: &str,
        file_extensions: &[String],
        existing_domains: &[String],
    ) -> DomainDetection {
        let summary_lower = folder_summary.to_lowercase();

        // Check file extensions
        let coding_exts = ["rs", "py", "js", "ts", "go", "java", "cpp"];
        let has_code_files = file_extensions
            .iter()
            .any(|ext| coding_exts.contains(&ext.as_str()));

        if has_code_files {
            let subcategory = self.detect_coding_subcategory(file_extensions, &summary_lower);
            let is_new = !existing_domains.contains(&"coding".to_string());
            return DomainDetection {
                domain: "coding".to_string(),
                subcategory,
                is_new_domain: is_new,
                confidence: 0.8,
            };
        }

        // Check for cooking content
        let cooking_keywords = ["recipe", "ingredient", "cook", "bake"];
        if cooking_keywords.iter().any(|kw| summary_lower.contains(kw)) {
            let is_new = !existing_domains.contains(&"cooking".to_string());
            return DomainDetection::new("cooking", 0.7)
                .with_subcategory("recipes")
                .as_new();
        }

        // Check for work content
        let work_keywords = ["meeting", "project", "deadline", "report"];
        if work_keywords.iter().any(|kw| summary_lower.contains(kw)) {
            let is_new = !existing_domains.contains(&"work".to_string());
            return DomainDetection {
                domain: "work".to_string(),
                subcategory: None,
                is_new_domain: is_new,
                confidence: 0.6,
            };
        }

        // Default to "other"
        DomainDetection::new("other", 0.3).as_new()
    }

    /// Detect coding subcategory from file extensions.
    fn detect_coding_subcategory(&self, extensions: &[String], _summary: &str) -> Option<String> {
        // Count extensions
        let rust_count = extensions.iter().filter(|e| *e == "rs").count();
        let python_count = extensions.iter().filter(|e| *e == "py").count();
        let js_count = extensions
            .iter()
            .filter(|e| *e == "js" || *e == "ts")
            .count();
        let go_count = extensions.iter().filter(|e| *e == "go").count();

        // Return most common language
        let max_count = rust_count.max(python_count).max(js_count).max(go_count);
        if max_count == 0 {
            return None;
        }

        if rust_count == max_count {
            Some("rust-projects".to_string())
        } else if python_count == max_count {
            Some("python-projects".to_string())
        } else if js_count == max_count {
            Some("javascript-projects".to_string())
        } else if go_count == max_count {
            Some("go-projects".to_string())
        } else {
            None
        }
    }

    /// Find relationships between nodes for cross-linking.
    pub async fn find_relationships(
        &self,
        node: &ContextNode,
        candidates: &[ContextNode],
    ) -> Result<Vec<RelatedNode>> {
        if self.is_llm_available() {
            self.find_relationships_with_llm(node, candidates).await
        } else if self.config.fallback_to_heuristic {
            Ok(self.find_relationships_heuristic(node, candidates))
        } else {
            Ok(Vec::new())
        }
    }

    /// Find relationships using LLM.
    async fn find_relationships_with_llm(
        &self,
        _node: &ContextNode,
        _candidates: &[ContextNode],
    ) -> Result<Vec<RelatedNode>> {
        // TODO: Implement LLM-based relationship finding
        info!("LLM relationship finding not yet implemented");
        Ok(Vec::new())
    }

    /// Find relationships using heuristics.
    fn find_relationships_heuristic(
        &self,
        node: &ContextNode,
        candidates: &[ContextNode],
    ) -> Vec<RelatedNode> {
        let mut relationships = Vec::new();

        for candidate in candidates {
            // Skip self
            if candidate.id == node.id {
                continue;
            }

            // Check for shared technologies
            let shared_techs: Vec<_> = node
                .entities
                .iter()
                .filter(|e| e.entity_type == EntityType::Technology)
                .filter(|e| {
                    candidate.entities.iter().any(|ce| {
                        ce.entity_type == EntityType::Technology
                            && ce.normalized_name == e.normalized_name
                    })
                })
                .collect();

            if !shared_techs.is_empty() {
                let strength = (shared_techs.len() as f32 * 0.2).min(0.8);
                relationships.push(
                    RelatedNode::new(
                        candidate.id.clone(),
                        CrossLinkType::SameTechnology,
                        strength,
                    )
                    .with_reason(format!(
                        "Shared technologies: {}",
                        shared_techs
                            .iter()
                            .map(|e| e.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                );
            }

            // Check for shared keywords
            let shared_keywords: Vec<_> = node
                .keywords
                .iter()
                .filter(|kw| candidate.keywords.contains(kw))
                .collect();

            if shared_keywords.len() >= 2 {
                let strength = (shared_keywords.len() as f32 * 0.15).min(0.7);
                relationships.push(
                    RelatedNode::new(candidate.id.clone(), CrossLinkType::SimilarTopic, strength)
                        .with_reason(format!(
                            "Shared keywords: {}",
                            shared_keywords
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                );
            }
        }

        relationships
    }

    /// Summarize a collection of child nodes into a parent summary.
    pub async fn summarize_children(&self, children: &[ContextNode]) -> Result<String> {
        if self.is_llm_available() {
            self.summarize_with_llm(children).await
        } else if self.config.fallback_to_heuristic {
            Ok(self.summarize_heuristic(children))
        } else {
            Ok(String::new())
        }
    }

    /// Summarize using LLM.
    async fn summarize_with_llm(&self, _children: &[ContextNode]) -> Result<String> {
        // TODO: Implement LLM-based summarization
        info!("LLM summarization not yet implemented");
        Ok(String::new())
    }

    /// Summarize using heuristics.
    fn summarize_heuristic(&self, children: &[ContextNode]) -> String {
        if children.is_empty() {
            return String::new();
        }

        // Collect unique keywords
        let mut all_keywords: Vec<String> =
            children.iter().flat_map(|c| c.keywords.clone()).collect();
        all_keywords.sort();
        all_keywords.dedup();

        // Count node types
        let project_count = children
            .iter()
            .filter(|c| c.node_type == NodeType::Project)
            .count();
        let doc_count = children
            .iter()
            .filter(|c| c.node_type == NodeType::Document)
            .count();
        let file_count = children
            .iter()
            .filter(|c| c.node_type == NodeType::FileReference)
            .count();

        // Build summary
        let mut parts = Vec::new();

        if project_count > 0 {
            parts.push(format!(
                "{} project{}",
                project_count,
                if project_count > 1 { "s" } else { "" }
            ));
        }
        if doc_count > 0 {
            parts.push(format!(
                "{} document{}",
                doc_count,
                if doc_count > 1 { "s" } else { "" }
            ));
        }
        if file_count > 0 {
            parts.push(format!(
                "{} file{}",
                file_count,
                if file_count > 1 { "s" } else { "" }
            ));
        }

        let mut summary = format!("Contains {}", parts.join(", "));

        if !all_keywords.is_empty() {
            let keywords_str = all_keywords
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            summary.push_str(&format!(". Topics: {}", keywords_str));
        }

        summary
    }
}

use crate::node::NodeType;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_analyze_document_heuristic() {
        let analyzer = LlmAnalyzer::heuristic_only();
        let content = r#"
# My Rust Project

This is a web server built with Rust and tokio.
It handles HTTP requests and connects to a PostgreSQL database.
"#;

        let context = AnalysisContext {
            file_extension: Some("rs".to_string()),
            ..Default::default()
        };

        let analysis = analyzer.analyze_document(content, &context).await.unwrap();

        assert!(!analysis.summary.is_empty());
        // Should detect coding domain from .rs extension
        assert!(
            analysis.suggested_domain == Some("coding".to_string()),
            "Expected coding domain, got {:?}",
            analysis.suggested_domain
        );
        assert!(analysis.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_detect_domain_coding() {
        let analyzer = LlmAnalyzer::heuristic_only();

        let detection = analyzer
            .detect_domain(
                "A rust project with web server",
                &["rs".to_string(), "toml".to_string()],
                &[],
            )
            .await
            .unwrap();

        assert_eq!(detection.domain, "coding");
        assert_eq!(detection.subcategory, Some("rust-projects".to_string()));
        assert!(detection.is_new_domain);
    }

    #[tokio::test]
    async fn test_detect_domain_cooking() {
        let analyzer = LlmAnalyzer::heuristic_only();

        let detection = analyzer
            .detect_domain(
                "Recipe for chocolate cake with ingredients",
                &["md".to_string()],
                &[],
            )
            .await
            .unwrap();

        assert_eq!(detection.domain, "cooking");
    }

    #[tokio::test]
    async fn test_detect_domain_work() {
        let analyzer = LlmAnalyzer::heuristic_only();

        let detection = analyzer
            .detect_domain(
                "Meeting notes and project deadline report",
                &["md".to_string(), "txt".to_string()],
                &[],
            )
            .await
            .unwrap();

        assert_eq!(detection.domain, "work");
    }

    #[test]
    fn test_extract_topics() {
        let analyzer = LlmAnalyzer::heuristic_only();
        let entities = vec![];

        let content = "This is an async API server with database support";
        let topics = analyzer.extract_topics(&entities, content);

        assert!(topics.contains(&"async-programming".to_string()));
        assert!(topics.contains(&"api".to_string()));
        assert!(topics.contains(&"database".to_string()));
    }

    #[tokio::test]
    async fn test_find_relationships() {
        let analyzer = LlmAnalyzer::heuristic_only();

        let mut node1 = ContextNode::project("project1", PathBuf::from("/p1"));
        node1.add_entity(Entity::new("Rust", EntityType::Technology, 0.9));
        node1.add_keyword("web");
        node1.add_keyword("server");

        let mut node2 = ContextNode::project("project2", PathBuf::from("/p2"));
        node2.add_entity(Entity::new("Rust", EntityType::Technology, 0.9));
        node2.add_keyword("cli");
        node2.add_keyword("server");

        let relationships = analyzer.find_relationships(&node1, &[node2]).await.unwrap();

        assert!(!relationships.is_empty());
        // Should find shared technology (Rust)
        assert!(
            relationships
                .iter()
                .any(|r| r.relationship == CrossLinkType::SameTechnology)
        );
    }

    #[tokio::test]
    async fn test_summarize_children() {
        let analyzer = LlmAnalyzer::heuristic_only();

        let mut child1 = ContextNode::project("project1", PathBuf::from("/p1"));
        child1.add_keyword("rust");
        child1.add_keyword("web");

        let mut child2 = ContextNode::document("readme", PathBuf::from("/readme.md"));
        child2.add_keyword("documentation");

        let summary = analyzer
            .summarize_children(&[child1, child2])
            .await
            .unwrap();

        assert!(summary.contains("project"));
        assert!(summary.contains("document"));
        assert!(summary.contains("Topics:"));
    }

    #[test]
    fn test_generate_summary_short_content() {
        let analyzer = LlmAnalyzer::heuristic_only();
        let content = "Just a brief note";

        let summary =
            analyzer.generate_heuristic_summary(content, &Some("/notes/test.md".to_string()));

        assert!(!summary.is_empty());
        assert!(summary.contains("test.md") || summary.contains("brief note"));
    }

    #[test]
    fn test_generate_summary_long_content() {
        let analyzer = LlmAnalyzer::heuristic_only();
        let content =
            "This is a very long document that contains a lot of information. ".repeat(50);

        let summary = analyzer.generate_heuristic_summary(&content, &None);

        assert!(summary.len() <= 310); // 300 + "..."
        assert!(summary.ends_with("..."));
    }
}
