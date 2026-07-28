//! # Context File System
//!
//! This crate implements the persistent memory system for Codex. It provides:
//!
//! - **Context Files**: Self-organizing knowledge units that the AI maintains
//! - **Concept Extraction**: Automatic identification of key concepts from conversations
//! - **Semantic Retrieval**: Search engine-style retrieval of relevant context
//! - **Bidirectional Sync**: Perfect synchronization between files, UI, and AI knowledge
//! - **Context Generation Pipeline**: Automatic extraction of entities, relationships,
//!   and context files from documents
//! - **Agentic Context System**: AI-powered hierarchical knowledge tree with automatic
//!   domain detection, cross-linking, and optimization
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Context File System                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ContextStore ──► ContextFile ──► ContentReference              │
//! │       │                │                                        │
//! │       ▼                ▼                                        │
//! │  ConceptExtractor  ConceptIndex  ◄── SemanticRetrieval         │
//! └─────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  Context Generation Pipeline                    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Document ──► SemanticChunker ──► EntityExtractor               │
//! │                                         │                       │
//! │                                         ▼                       │
//! │  ContextGenerator ◄── RelationshipExtractor                    │
//! │       │                                                         │
//! │       ▼                                                         │
//! │  GeneratedContext ──► ContextFile                              │
//! └─────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  Agentic Context System                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Folder ──► ContextAgent ──► LlmAnalyzer                       │
//! │                  │                │                             │
//! │                  ▼                ▼                             │
//! │            ContextTree ◄── DomainDetection                     │
//! │                  │                                              │
//! │                  ▼                                              │
//! │  TreeStore ◄── TreeOptimizer ──► CrossLinks                    │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

// Core modules
pub mod concept;
pub mod context_file;
pub mod error;
pub mod extraction;
pub mod index;
pub mod query;
pub mod retrieval;
pub mod storage;
pub mod sync;

// Context generation pipeline modules
pub mod chunker;
pub mod entity;
pub mod generator;
pub mod pipeline;
pub mod relationship;

// Agentic context system modules
pub mod agent;
pub mod llm;
pub mod node;
pub mod optimizer;
pub mod tree;
pub mod tree_storage;

// Core re-exports
pub use concept::Concept;
pub use concept::ConceptRelation;
pub use concept::RelationType;
pub use context_file::ContentReference;
pub use context_file::ContextFile;
pub use context_file::ContextMetadata;
pub use context_file::ReferenceType;
pub use error::ContextError;
pub use error::Result;
pub use extraction::ConceptExtractor;
pub use index::ConceptIndex;
pub use query::Query;
pub use query::QueryIntent;
pub use query::QueryResult;
pub use retrieval::RetrievalEngine;
pub use storage::ContextStore;
pub use sync::SyncManager;

// Pipeline re-exports
pub use chunker::Chunk;
pub use chunker::ChunkMetadata;
pub use chunker::ChunkType;
pub use chunker::ChunkerConfig;
pub use chunker::SemanticChunker;
pub use entity::Entity;
pub use entity::EntityExtractor;
pub use entity::EntityExtractorConfig;
pub use entity::EntityMention;
pub use entity::EntityType;
pub use generator::ClusterMethod;
pub use generator::ContextGenerator;
pub use generator::EntityCluster;
pub use generator::GeneratedContext;
pub use generator::GeneratorConfig;
pub use pipeline::ContextPipeline;
pub use pipeline::DocumentResult;
pub use pipeline::PipelineBuilder;
pub use pipeline::PipelineConfig;
pub use pipeline::PipelineResult;
pub use pipeline::PipelineStats;
pub use relationship::EvidenceType;
pub use relationship::Relationship;
pub use relationship::RelationshipEvidence;
pub use relationship::RelationshipExtractor;
pub use relationship::RelationshipExtractorConfig;
pub use relationship::RelationshipType;

// Agentic system re-exports
pub use agent::AgentBuilder;
pub use agent::AgentConfig;
pub use agent::AgentQueryResult;
pub use agent::ContextAgent;
pub use agent::ProcessingResult;
pub use llm::AnalysisContext;
pub use llm::LlmAnalyzer;
pub use llm::LlmConfig;
pub use node::ContextNode;
pub use node::CrossLinkType;
pub use node::DocumentAnalysis;
pub use node::DomainDetection;
pub use node::NodeType;
pub use node::RelatedNode;
pub use optimizer::OptimizationAnalysis;
pub use optimizer::OptimizationResult;
pub use optimizer::OptimizerConfig;
pub use optimizer::TreeOptimizer;
pub use tree::ContextTree;
pub use tree::TreeStats;
pub use tree_storage::TreeStore;
pub use tree_storage::TreeVisualization;
