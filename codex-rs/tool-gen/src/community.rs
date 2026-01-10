//! Community tool sharing features.
//!
//! The `CommunityHub` handles publishing, discovering, and installing
//! tools from the community repository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{Result, ToolError};
use crate::tool::Tool;

/// A tool that has been shared with the community.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTool {
    /// Unique share ID.
    pub share_id: String,

    /// The tool data.
    pub tool: Tool,

    /// Publisher information.
    pub publisher: PublisherInfo,

    /// When the tool was published.
    pub published_at: DateTime<Utc>,

    /// Number of downloads.
    pub downloads: u64,

    /// Average rating.
    pub rating: f32,

    /// Number of ratings.
    pub rating_count: u64,

    /// Reviews from users.
    pub reviews: Vec<ToolReview>,
}

/// Information about a tool publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    /// Publisher ID.
    pub id: String,

    /// Display name.
    pub name: String,

    /// Whether the publisher is verified.
    pub verified: bool,

    /// Number of tools published.
    pub tool_count: u64,
}

/// A review of a shared tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReview {
    /// Reviewer ID.
    pub reviewer_id: String,

    /// Reviewer display name.
    pub reviewer_name: String,

    /// Rating (1-5).
    pub rating: u8,

    /// Review text.
    pub text: String,

    /// When the review was posted.
    pub posted_at: DateTime<Utc>,

    /// Whether this review was helpful (upvotes).
    pub helpful_count: u64,
}

/// A rating for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRating {
    /// Tool share ID.
    pub tool_id: String,

    /// User ID who rated.
    pub user_id: String,

    /// Rating value (1-5).
    pub rating: u8,

    /// When the rating was given.
    pub rated_at: DateTime<Utc>,
}

impl ToolRating {
    /// Create a new rating.
    pub fn new(tool_id: impl Into<String>, user_id: impl Into<String>, rating: u8) -> Self {
        Self {
            tool_id: tool_id.into(),
            user_id: user_id.into(),
            rating: rating.clamp(1, 5),
            rated_at: Utc::now(),
        }
    }
}

/// Hub for community tool sharing.
///
/// The hub provides APIs for:
/// - Publishing tools to the community
/// - Discovering and searching community tools
/// - Installing community tools locally
/// - Rating and reviewing tools
pub struct CommunityHub {
    /// Base URL for the community API.
    api_url: String,

    /// User authentication token.
    auth_token: Option<String>,

    /// Local cache of discovered tools.
    cache: Vec<SharedTool>,
}

impl CommunityHub {
    /// Create a new community hub.
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            auth_token: None,
            cache: Vec::new(),
        }
    }

    /// Set the authentication token.
    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Publish a tool to the community.
    pub async fn publish(&self, tool: &Tool) -> Result<SharedTool> {
        if self.auth_token.is_none() {
            return Err(ToolError::Network(
                "Authentication required to publish tools".to_string(),
            ));
        }

        // Validate tool before publishing
        self.validate_for_publish(tool)?;

        // This is a placeholder - actual implementation would call the API
        info!("Publishing tool: {} to community", tool.name);

        // Simulate successful publish
        let shared = SharedTool {
            share_id: uuid::Uuid::new_v4().to_string(),
            tool: tool.clone(),
            publisher: PublisherInfo {
                id: "current_user".to_string(),
                name: "Current User".to_string(),
                verified: false,
                tool_count: 1,
            },
            published_at: Utc::now(),
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            reviews: Vec::new(),
        };

        Ok(shared)
    }

    /// Validate a tool for publishing.
    fn validate_for_publish(&self, tool: &Tool) -> Result<()> {
        if tool.name.is_empty() {
            return Err(ToolError::InvalidDefinition("Tool name is required".to_string()));
        }

        if tool.description.is_empty() {
            return Err(ToolError::InvalidDefinition(
                "Tool description is required".to_string(),
            ));
        }

        if tool.definition.implementation.is_empty() {
            return Err(ToolError::InvalidDefinition(
                "Tool implementation is required".to_string(),
            ));
        }

        // Security checks
        self.security_check(tool)?;

        Ok(())
    }

    /// Perform security checks on a tool.
    fn security_check(&self, tool: &Tool) -> Result<()> {
        let impl_lower = tool.definition.implementation.to_lowercase();

        // Check for obvious dangerous patterns
        let dangerous_patterns = [
            "rm -rf",
            "format c:",
            "del /f /s /q",
            "eval(",
            "exec(",
            "system(",
        ];

        for pattern in dangerous_patterns {
            if impl_lower.contains(pattern) {
                return Err(ToolError::SecurityValidation(format!(
                    "Tool contains potentially dangerous pattern: {pattern}"
                )));
            }
        }

        Ok(())
    }

    /// Search for community tools.
    pub async fn search(&self, query: &str) -> Result<Vec<SharedTool>> {
        debug!("Searching community tools: {query}");

        // This is a placeholder - actual implementation would call the API
        warn!("Community search not yet implemented");

        // Return cached tools that match
        let results: Vec<SharedTool> = self
            .cache
            .iter()
            .filter(|t| {
                t.tool.name.to_lowercase().contains(&query.to_lowercase())
                    || t.tool
                        .description
                        .to_lowercase()
                        .contains(&query.to_lowercase())
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get popular tools.
    pub async fn get_popular(&self, limit: usize) -> Result<Vec<SharedTool>> {
        debug!("Getting popular tools (limit: {limit})");

        // Placeholder - would call API
        let mut results = self.cache.clone();
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results.truncate(limit);

        Ok(results)
    }

    /// Get recently published tools.
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<SharedTool>> {
        debug!("Getting recent tools (limit: {limit})");

        // Placeholder - would call API
        let mut results = self.cache.clone();
        results.sort_by(|a, b| b.published_at.cmp(&a.published_at));
        results.truncate(limit);

        Ok(results)
    }

    /// Get top-rated tools.
    pub async fn get_top_rated(&self, limit: usize) -> Result<Vec<SharedTool>> {
        debug!("Getting top-rated tools (limit: {limit})");

        // Placeholder - would call API
        let mut results: Vec<_> = self
            .cache
            .iter()
            .filter(|t| t.rating_count >= 5) // Minimum ratings
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        Ok(results)
    }

    /// Install a tool from the community.
    pub async fn install(&self, share_id: &str) -> Result<Tool> {
        debug!("Installing tool: {share_id}");

        // Find in cache (placeholder for API call)
        let shared = self
            .cache
            .iter()
            .find(|t| t.share_id == share_id)
            .ok_or_else(|| ToolError::NotFound(share_id.to_string()))?;

        info!("Installing community tool: {}", shared.tool.name);

        // Return a copy of the tool
        Ok(shared.tool.clone())
    }

    /// Rate a community tool.
    pub async fn rate(&self, rating: ToolRating) -> Result<()> {
        if self.auth_token.is_none() {
            return Err(ToolError::Network(
                "Authentication required to rate tools".to_string(),
            ));
        }

        debug!("Rating tool {} with {}", rating.tool_id, rating.rating);

        // Placeholder - would call API
        info!(
            "Rated tool {} with {} stars",
            rating.tool_id, rating.rating
        );

        Ok(())
    }

    /// Submit a review for a community tool.
    pub async fn review(&self, share_id: &str, rating: u8, text: &str) -> Result<ToolReview> {
        if self.auth_token.is_none() {
            return Err(ToolError::Network(
                "Authentication required to review tools".to_string(),
            ));
        }

        debug!("Reviewing tool: {share_id}");

        // Placeholder - would call API
        let review = ToolReview {
            reviewer_id: "current_user".to_string(),
            reviewer_name: "Current User".to_string(),
            rating: rating.clamp(1, 5),
            text: text.to_string(),
            posted_at: Utc::now(),
            helpful_count: 0,
        };

        info!("Submitted review for tool: {share_id}");

        Ok(review)
    }

    /// Fork a community tool for customization.
    pub async fn fork(&self, share_id: &str) -> Result<Tool> {
        let mut tool = self.install(share_id).await?;

        // Create a new ID for the fork
        tool.id = uuid::Uuid::new_v4().to_string();
        tool.name = format!("{}-fork", tool.name);
        tool.author = "user".to_string();
        tool.sharing.is_public = false;
        tool.sharing.share_id = None;

        info!("Forked tool: {share_id} as {}", tool.name);

        Ok(tool)
    }
}

impl Default for CommunityHub {
    fn default() -> Self {
        Self::new("https://api.codex-tools.example.com")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCategory;

    #[test]
    fn test_rating_clamping() {
        let rating = ToolRating::new("tool1", "user1", 10);
        assert_eq!(rating.rating, 5);

        let rating = ToolRating::new("tool1", "user1", 0);
        assert_eq!(rating.rating, 1);
    }

    #[tokio::test]
    async fn test_security_check() {
        let hub = CommunityHub::default();
        let mut tool = Tool::new("dangerous", "Test", ToolCategory::Utility);

        // Safe tool should pass
        assert!(hub.security_check(&tool).is_ok());

        // Dangerous pattern should fail
        tool.definition.implementation = "rm -rf /".to_string();
        assert!(hub.security_check(&tool).is_err());
    }
}
