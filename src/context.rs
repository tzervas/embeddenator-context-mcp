//! Context data structures and core types
//!
//! Inspired by memory-gate's LearningContext pattern with enhancements
//! for temporal reasoning and MCP integration.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use base64::Engine;

/// Unique identifier for a context entry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ContextId(pub String);

impl ContextId {
    /// Generate a new random context ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Generate a deterministic ID from content hash
    pub fn from_content(content: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        Self(base64::engine::general_purpose::STANDARD.encode(&hash[..16]))
    }

    /// Create from a string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ContextId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Domain classification for context entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDomain {
    /// General purpose context
    General,
    /// Code and programming related
    Code,
    /// Documentation and technical writing
    Documentation,
    /// Conversation history
    Conversation,
    /// File system operations
    Filesystem,
    /// Web search results
    WebSearch,
    /// Dataset information
    Dataset,
    /// Research and papers
    Research,
    /// Custom domain with identifier
    Custom(String),
}

impl Default for ContextDomain {
    fn default() -> Self {
        Self::General
    }
}

/// Metadata associated with a context entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// Source of the context (e.g., "user", "web", "file")
    #[serde(default)]
    pub source: String,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Importance score (0.0 to 1.0)
    #[serde(default = "default_importance")]
    pub importance: f32,

    /// Whether this context has been verified/screened
    #[serde(default)]
    pub verified: bool,

    /// Security screening status
    #[serde(default)]
    pub screening_status: ScreeningStatus,

    /// Custom key-value pairs
    #[serde(default)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

fn default_importance() -> f32 {
    1.0
}

impl Default for ContextMetadata {
    fn default() -> Self {
        Self {
            source: String::new(),
            tags: Vec::new(),
            importance: 1.0,
            verified: false,
            screening_status: ScreeningStatus::Unscreened,
            custom: std::collections::HashMap::new(),
        }
    }
}

/// Security screening status for context entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreeningStatus {
    /// Not yet screened
    #[default]
    Unscreened,
    /// Screened and safe
    Safe,
    /// Screened and flagged for review
    Flagged,
    /// Screened and blocked
    Blocked,
    /// Screening in progress
    Pending,
}

/// A context entry for storage and retrieval
///
/// Inspired by memory-gate's LearningContext with additions for:
/// - Temporal reasoning (created_at, accessed_at, expires_at)
/// - Security screening integration
/// - RAG-optimized fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Unique identifier
    pub id: ContextId,

    /// Main content of the context
    pub content: String,

    /// Domain classification
    pub domain: ContextDomain,

    /// When this context was created
    pub created_at: DateTime<Utc>,

    /// When this context was last accessed
    pub accessed_at: DateTime<Utc>,

    /// Optional expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    /// Associated metadata
    pub metadata: ContextMetadata,

    /// Optional embedding vector for similarity search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl Context {
    /// Create a new context entry
    pub fn new(content: impl Into<String>, domain: ContextDomain) -> Self {
        let content = content.into();
        let now = Utc::now();
        Self {
            id: ContextId::from_content(&content),
            content,
            domain,
            created_at: now,
            accessed_at: now,
            expires_at: None,
            metadata: ContextMetadata::default(),
            embedding: None,
        }
    }

    /// Create with a specific ID
    pub fn with_id(mut self, id: ContextId) -> Self {
        self.id = id;
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: ContextMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set source in metadata
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = source.into();
        self
    }

    /// Set importance
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.metadata.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags = tags;
        self
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set embedding vector
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set TTL (time to live)
    pub fn with_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.expires_at = Some(Utc::now() + Duration::from_std(ttl).unwrap_or(Duration::hours(24)));
        self
    }

    /// Check if context has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get age in hours (useful for temporal reasoning)
    pub fn age_hours(&self) -> f64 {
        self.age_seconds() as f64 / 3600.0
    }

    /// Mark as accessed (updates accessed_at)
    pub fn mark_accessed(&mut self) {
        self.accessed_at = Utc::now();
    }

    /// Check if context is safe to use (screened)
    pub fn is_safe(&self) -> bool {
        matches!(
            self.metadata.screening_status,
            ScreeningStatus::Safe | ScreeningStatus::Unscreened
        )
    }
}

/// Builder for creating context queries
#[derive(Debug, Clone, Default)]
pub struct ContextQuery {
    /// Text query for similarity search
    pub query: Option<String>,
    /// Filter by domain
    pub domain_filter: Option<ContextDomain>,
    /// Filter by tags (any match)
    pub tag_filter: Option<Vec<String>>,
    /// Filter by source
    pub source_filter: Option<String>,
    /// Minimum importance threshold
    pub min_importance: Option<f32>,
    /// Maximum age in seconds
    pub max_age_seconds: Option<i64>,
    /// Only return verified/screened context
    pub verified_only: bool,
    /// Maximum results to return
    pub limit: usize,
}

impl ContextQuery {
    pub fn new() -> Self {
        Self {
            limit: 10,
            ..Default::default()
        }
    }

    pub fn with_text(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_domain(mut self, domain: ContextDomain) -> Self {
        self.domain_filter = Some(domain);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tag_filter = Some(tags);
        self
    }

    pub fn with_min_importance(mut self, importance: f32) -> Self {
        self.min_importance = Some(importance);
        self
    }

    pub fn with_max_age(mut self, seconds: i64) -> Self {
        self.max_age_seconds = Some(seconds);
        self
    }

    pub fn with_max_age_hours(mut self, hours: i64) -> Self {
        self.max_age_seconds = Some(hours * 3600);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        if self.tag_filter.is_none() {
            self.tag_filter = Some(Vec::new());
        }
        self.tag_filter.as_mut().unwrap().push(tag);
        self
    }

    pub fn verified_only(mut self) -> Self {
        self.verified_only = true;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new("Test content", ContextDomain::Code);
        assert!(!ctx.content.is_empty());
        assert_eq!(ctx.domain, ContextDomain::Code);
        assert!(!ctx.is_expired());
    }

    #[test]
    fn test_context_id_from_content() {
        let id1 = ContextId::from_content("hello world");
        let id2 = ContextId::from_content("hello world");
        let id3 = ContextId::from_content("different content");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_context_age() {
        let ctx = Context::new("Test", ContextDomain::General);
        assert!(ctx.age_seconds() >= 0);
        assert!(ctx.age_hours() >= 0.0);
    }

    #[test]
    fn test_context_query_builder() {
        let query = ContextQuery::new()
            .with_text("search term")
            .with_domain(ContextDomain::Code)
            .with_min_importance(0.5)
            .with_limit(20);

        assert_eq!(query.query, Some("search term".to_string()));
        assert_eq!(query.domain_filter, Some(ContextDomain::Code));
        assert_eq!(query.min_importance, Some(0.5));
        assert_eq!(query.limit, 20);
    }
}
