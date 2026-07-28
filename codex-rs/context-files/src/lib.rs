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
pub use concept::{Concept, ConceptRelation, RelationType};
pub use context_file::{ContentReference, ContextFile, ContextMetadata, ReferenceType};
pub use error::{ContextError, Result};
pub use extraction::ConceptExtractor;
pub use index::ConceptIndex;
pub use query::{Query, QueryIntent, QueryResult};
pub use retrieval::RetrievalEngine;
pub use storage::ContextStore;
pub use sync::SyncManager;

// Pipeline re-exports
pub use chunker::{Chunk, ChunkMetadata, ChunkType, ChunkerConfig, SemanticChunker};
pub use entity::{Entity, EntityExtractor, EntityExtractorConfig, EntityMention, EntityType};
pub use generator::{
    ClusterMethod, ContextGenerator, EntityCluster, GeneratedContext, GeneratorConfig,
};
pub use pipeline::{
    ContextPipeline, DocumentResult, PipelineBuilder, PipelineConfig, PipelineResult, PipelineStats,
};
pub use relationship::{
    EvidenceType, Relationship, RelationshipEvidence, RelationshipExtractor,
    RelationshipExtractorConfig, RelationshipType,
};

// Agentic system re-exports
pub use agent::{AgentBuilder, AgentConfig, AgentQueryResult, ContextAgent, ProcessingResult};
pub use llm::{AnalysisContext, LlmAnalyzer, LlmConfig};
pub use node::{
    ContextNode, CrossLinkType, DocumentAnalysis, DomainDetection, NodeType, RelatedNode,
};
pub use optimizer::{OptimizationAnalysis, OptimizationResult, OptimizerConfig, TreeOptimizer};
pub use tree::{ContextTree, TreeStats};
pub use tree_storage::{TreeStore, TreeVisualization};
