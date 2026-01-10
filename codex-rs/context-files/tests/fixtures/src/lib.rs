//! # DataFlow
//!
//! A high-performance data processing pipeline.
//!
//! Created by Alice Johnson at TechCorp Inc.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod processor;
pub mod storage;

/// Error types for DataFlow operations.
#[derive(Error, Debug)]
pub enum DataFlowError {
    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for DataFlow operations.
pub type Result<T> = std::result::Result<T, DataFlowError>;

/// Represents a data event in the pipeline.
///
/// Events are the primary unit of data flowing through the system.
/// Each event has a unique ID, timestamp, and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for the event.
    pub id: Uuid,

    /// ISO 8601 timestamp when the event was created.
    pub timestamp: String,

    /// Event type for routing purposes.
    pub event_type: String,

    /// The actual data payload.
    pub payload: serde_json::Value,

    /// Source system that generated the event.
    pub source: String,
}

impl Event {
    /// Create a new event with the given type and payload.
    pub fn new(event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.into(),
            payload,
            source: "dataflow".to_string(),
        }
    }
}

/// Configuration for the DataFlow pipeline.
///
/// This struct holds all configuration options for the system.
/// See `config.toml` for example configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// PostgreSQL connection string.
    pub database_url: String,

    /// Redis connection string.
    pub redis_url: String,

    /// Kafka broker addresses.
    pub kafka_brokers: Vec<String>,

    /// Number of worker threads.
    pub worker_count: usize,

    /// Maximum batch size for processing.
    pub batch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgres://localhost/dataflow".to_string(),
            redis_url: "redis://localhost".to_string(),
            kafka_brokers: vec!["localhost:9092".to_string()],
            worker_count: 4,
            batch_size: 100,
        }
    }
}

/// The main processor trait that all data processors must implement.
///
/// Implemented by `DataProcessor` and `BatchProcessor`.
pub trait Processor: Send + Sync {
    /// Process a single event.
    fn process(&self, event: &Event) -> Result<Event>;

    /// Process a batch of events.
    fn process_batch(&self, events: &[Event]) -> Result<Vec<Event>> {
        events.iter().map(|e| self.process(e)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("test", serde_json::json!({"key": "value"}));
        assert_eq!(event.event_type, "test");
        assert_eq!(event.source, "dataflow");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.worker_count, 4);
        assert_eq!(config.batch_size, 100);
    }
}
