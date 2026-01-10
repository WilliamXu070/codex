//! Embedding providers.
//!
//! Supports multiple embedding providers including OpenAI and local models.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{EmbeddingError, Result};
use crate::Embedding;

/// Request for generating embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// Text to embed.
    pub text: String,

    /// Model to use (provider-specific).
    pub model: Option<String>,

    /// Dimensions for the output (if supported by provider).
    pub dimensions: Option<usize>,
}

impl EmbeddingRequest {
    /// Create a new embedding request.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            model: None,
            dimensions: None,
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the output dimensions.
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }
}

/// Response from embedding generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The generated embedding.
    pub embedding: Embedding,

    /// Model used to generate the embedding.
    pub model: String,

    /// Dimension of the embedding.
    pub dimension: usize,

    /// Token usage (if available).
    pub tokens_used: Option<u64>,
}

/// Trait for embedding providers.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the default model for this provider.
    fn default_model(&self) -> &str;

    /// Get the default embedding dimension.
    fn default_dimension(&self) -> usize;

    /// Generate an embedding for the given text.
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Generate embeddings for multiple texts.
    async fn embed_batch(&self, requests: Vec<EmbeddingRequest>) -> Result<Vec<EmbeddingResponse>> {
        // Default implementation: process sequentially
        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            results.push(self.embed(request).await?);
        }
        Ok(results)
    }

    /// Check if the provider is available (API key set, etc.).
    fn is_available(&self) -> bool;
}

/// OpenAI embedding provider.
pub struct OpenAIProvider {
    /// API key.
    api_key: Option<String>,

    /// API base URL.
    base_url: String,

    /// HTTP client.
    client: reqwest::Client,

    /// Default model.
    default_model: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider.
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
            default_model: "text-embedding-3-small".to_string(),
        }
    }

    /// Set the API key.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn default_dimension(&self) -> usize {
        match self.default_model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        }
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(EmbeddingError::ProviderNotConfigured)?;

        let model = request.model.unwrap_or_else(|| self.default_model.clone());

        debug!("Generating embedding with model: {model}");

        // Build the request body
        let mut body = serde_json::json!({
            "input": request.text,
            "model": model
        });

        if let Some(dims) = request.dimensions {
            body["dimensions"] = serde_json::json!(dims);
        }

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);

            return Err(EmbeddingError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ApiRequest(format!(
                "API error: {error_text}"
            )));
        }

        let result: OpenAIEmbeddingResponse = response.json().await?;

        let embedding = result
            .data
            .first()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embedding in response".to_string()))?
            .embedding
            .clone();

        let dimension = embedding.len();
        let tokens_used = result.usage.map(|u| u.total_tokens);

        info!("Generated embedding with {dimension} dimensions");

        Ok(EmbeddingResponse {
            embedding,
            model: result.model,
            dimension,
            tokens_used,
        })
    }

    async fn embed_batch(&self, requests: Vec<EmbeddingRequest>) -> Result<Vec<EmbeddingResponse>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let api_key = self
            .api_key
            .as_ref()
            .ok_or(EmbeddingError::ProviderNotConfigured)?;

        let model = requests[0]
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let texts: Vec<&str> = requests.iter().map(|r| r.text.as_str()).collect();

        debug!(
            "Generating batch embeddings for {} texts with model: {model}",
            texts.len()
        );

        let body = serde_json::json!({
            "input": texts,
            "model": model
        });

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ApiRequest(format!(
                "API error: {error_text}"
            )));
        }

        let result: OpenAIEmbeddingResponse = response.json().await?;

        let responses: Vec<EmbeddingResponse> = result
            .data
            .into_iter()
            .map(|item| {
                let dimension = item.embedding.len();
                EmbeddingResponse {
                    embedding: item.embedding,
                    model: result.model.clone(),
                    dimension,
                    tokens_used: None,
                }
            })
            .collect();

        info!("Generated {} batch embeddings", responses.len());

        Ok(responses)
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}

/// OpenAI API response format.
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    #[allow(dead_code)]
    prompt_tokens: u64,
    total_tokens: u64,
}

/// Local embedding provider (placeholder for future implementation).
pub struct LocalProvider {
    model_path: Option<String>,
}

impl LocalProvider {
    /// Create a new local provider.
    pub fn new() -> Self {
        Self { model_path: None }
    }

    /// Set the model path.
    pub fn with_model_path(mut self, path: impl Into<String>) -> Self {
        self.model_path = Some(path.into());
        self
    }
}

impl Default for LocalProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingProvider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    fn default_model(&self) -> &str {
        "all-MiniLM-L6-v2"
    }

    fn default_dimension(&self) -> usize {
        384 // MiniLM dimension
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Placeholder - actual implementation would use a local model
        warn!("Local embedding not yet implemented, returning dummy embedding");

        let dimension = self.default_dimension();
        let embedding = vec![0.0f32; dimension];

        Ok(EmbeddingResponse {
            embedding,
            model: self.default_model().to_string(),
            dimension,
            tokens_used: Some(request.text.split_whitespace().count() as u64),
        })
    }

    fn is_available(&self) -> bool {
        self.model_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_request() {
        let request = EmbeddingRequest::new("Hello world")
            .with_model("text-embedding-3-small")
            .with_dimensions(512);

        assert_eq!(request.text, "Hello world");
        assert_eq!(request.model, Some("text-embedding-3-small".to_string()));
        assert_eq!(request.dimensions, Some(512));
    }

    #[test]
    fn test_openai_provider_default_dimensions() {
        let provider = OpenAIProvider::new().with_model("text-embedding-3-large");
        assert_eq!(provider.default_dimension(), 3072);
    }
}
