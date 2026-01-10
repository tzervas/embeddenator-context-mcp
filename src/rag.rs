//! CPU-optimized RAG processing for context retrieval
//!
//! Provides parallel processing capabilities for efficient
//! retrieval-augmented generation operations on screened safe inputs.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::context::{Context, ContextDomain, ContextQuery};
use crate::error::ContextResult;
use crate::storage::ContextStore;
use crate::temporal::{TemporalQuery, TemporalStats};

/// RAG processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Maximum results per query
    pub max_results: usize,
    /// Minimum relevance threshold (0.0 to 1.0)
    pub min_relevance: f64,
    /// Enable parallel processing
    pub parallel: bool,
    /// Number of threads (0 = auto)
    pub num_threads: usize,
    /// Apply temporal decay to scoring
    pub temporal_decay: bool,
    /// Only retrieve screened-safe contexts
    pub safe_only: bool,
    /// Chunk size for parallel processing
    pub chunk_size: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_relevance: 0.1,
            parallel: true,
            num_threads: 0, // Auto-detect
            temporal_decay: true,
            safe_only: true,
            chunk_size: 1000,
        }
    }
}

/// Result from RAG retrieval with scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredContext {
    /// The context
    pub context: Context,
    /// Relevance score (0.0 to 1.0)
    pub score: f64,
    /// Contributing score components
    pub score_breakdown: ScoreBreakdown,
}

/// Breakdown of score components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Temporal relevance
    pub temporal: f64,
    /// Importance score
    pub importance: f64,
    /// Domain match score
    pub domain_match: f64,
    /// Tag match score
    pub tag_match: f64,
    /// Content similarity (if embedding available)
    pub similarity: Option<f64>,
}

/// RAG retrieval results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Scored contexts
    pub contexts: Vec<ScoredContext>,
    /// Query used
    pub query_summary: String,
    /// Processing time in ms
    pub processing_time_ms: u64,
    /// Total candidates considered
    pub candidates_considered: usize,
    /// Temporal statistics
    pub temporal_stats: TemporalStats,
}

/// CPU-optimized RAG processor
pub struct RagProcessor {
    config: RagConfig,
    store: Arc<ContextStore>,
}

impl RagProcessor {
    /// Create a new RAG processor
    pub fn new(store: Arc<ContextStore>, config: RagConfig) -> Self {
        // Configure thread pool if specified
        if config.num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(config.num_threads)
                .build_global()
                .ok();
        }

        Self { config, store }
    }

    /// Create with default configuration
    pub fn with_defaults(store: Arc<ContextStore>) -> Self {
        Self::new(store, RagConfig::default())
    }

    /// Retrieve contexts using a query
    pub async fn retrieve(&self, query: &RetrievalQuery) -> ContextResult<RetrievalResult> {
        let start = std::time::Instant::now();

        // Build context query
        let mut ctx_query = ContextQuery::new();
        
        if let Some(domain) = &query.domain {
            ctx_query = ctx_query.with_domain(domain.clone());
        }
        
        for tag in &query.tags {
            ctx_query = ctx_query.with_tag(tag.clone());
        }

        if let Some(min_importance) = query.min_importance {
            ctx_query = ctx_query.with_min_importance(min_importance);
        }

        // Get candidates from storage
        let candidates: Vec<Context> = self.store.query(&ctx_query).await?;
        let candidates_count = candidates.len();

        // Apply temporal filtering
        let temporal_query = query.temporal.clone().unwrap_or_default();
        let filtered: Vec<Context> = candidates
            .into_iter()
            .filter(|c| temporal_query.matches(c))
            .filter(|c| !self.config.safe_only || c.is_safe())
            .collect();

        // Score contexts (parallel or sequential)
        let scored = if self.config.parallel && filtered.len() > self.config.chunk_size {
            self.score_parallel(&filtered, query, &temporal_query)
        } else {
            self.score_sequential(&filtered, query, &temporal_query)
        };

        // Filter by minimum relevance and sort
        let mut results: Vec<ScoredContext> = scored
            .into_iter()
            .filter(|s| s.score >= self.config.min_relevance)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.config.max_results);

        let temporal_stats = TemporalStats::from_contexts(
            &results.iter().map(|s| s.context.clone()).collect::<Vec<_>>()
        );

        Ok(RetrievalResult {
            contexts: results,
            query_summary: query.to_string(),
            processing_time_ms: start.elapsed().as_millis() as u64,
            candidates_considered: candidates_count,
            temporal_stats,
        })
    }

    /// Score contexts in parallel using rayon
    fn score_parallel(
        &self,
        contexts: &[Context],
        query: &RetrievalQuery,
        temporal: &TemporalQuery,
    ) -> Vec<ScoredContext> {
        contexts
            .par_iter()
            .map(|ctx| self.score_context(ctx, query, temporal))
            .collect()
    }

    /// Score contexts sequentially
    fn score_sequential(
        &self,
        contexts: &[Context],
        query: &RetrievalQuery,
        temporal: &TemporalQuery,
    ) -> Vec<ScoredContext> {
        contexts
            .iter()
            .map(|ctx| self.score_context(ctx, query, temporal))
            .collect()
    }

    /// Score a single context
    fn score_context(
        &self,
        ctx: &Context,
        query: &RetrievalQuery,
        temporal: &TemporalQuery,
    ) -> ScoredContext {
        let mut breakdown = ScoreBreakdown::default();

        // Temporal score
        breakdown.temporal = if self.config.temporal_decay {
            temporal.relevance_score(ctx)
        } else {
            1.0
        };

        // Importance score
        breakdown.importance = ctx.metadata.importance as f64;

        // Domain match score
        breakdown.domain_match = if query.domain.as_ref() == Some(&ctx.domain) {
            1.0
        } else if query.domain.is_none() {
            0.5 // Neutral if no domain specified
        } else {
            0.2 // Partial credit for different domains
        };

        // Tag match score
        if !query.tags.is_empty() {
            let matching_tags = query
                .tags
                .iter()
                .filter(|t| ctx.metadata.tags.contains(*t))
                .count();
            breakdown.tag_match = matching_tags as f64 / query.tags.len() as f64;
        } else {
            breakdown.tag_match = 0.5; // Neutral
        }

        // Content similarity (placeholder for embedding-based scoring)
        // In a full implementation, this would compute cosine similarity
        // between query embedding and context embedding
        breakdown.similarity = None;

        // Weighted final score
        let score = 0.25 * breakdown.temporal
            + 0.25 * breakdown.importance
            + 0.25 * breakdown.domain_match
            + 0.25 * breakdown.tag_match;

        ScoredContext {
            context: ctx.clone(),
            score,
            score_breakdown: breakdown,
        }
    }

    /// Retrieve by text query with simple keyword matching
    pub async fn retrieve_by_text(&self, text: &str) -> ContextResult<RetrievalResult> {
        let query = RetrievalQuery::from_text(text);
        self.retrieve(&query).await
    }

    /// Get configuration
    pub fn config(&self) -> &RagConfig {
        &self.config
    }
}

/// Query for RAG retrieval
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievalQuery {
    /// Text query (for keyword/semantic matching)
    pub text: Option<String>,
    /// Domain filter
    pub domain: Option<ContextDomain>,
    /// Tag filters
    pub tags: Vec<String>,
    /// Minimum importance
    pub min_importance: Option<f32>,
    /// Temporal query parameters
    pub temporal: Option<TemporalQuery>,
    /// Maximum results
    pub max_results: Option<usize>,
}

impl RetrievalQuery {
    /// Create a new retrieval query
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from text
    pub fn from_text(text: &str) -> Self {
        Self {
            text: Some(text.to_string()),
            ..Default::default()
        }
    }

    /// Set domain filter
    pub fn with_domain(mut self, domain: ContextDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Add tag filter
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set minimum importance
    pub fn with_min_importance(mut self, importance: f32) -> Self {
        self.min_importance = Some(importance);
        self
    }

    /// Set temporal parameters
    pub fn with_temporal(mut self, temporal: TemporalQuery) -> Self {
        self.temporal = Some(temporal);
        self
    }

    /// Query for recent contexts
    pub fn recent(hours: i64) -> Self {
        Self::new().with_temporal(TemporalQuery::recent(hours))
    }
}

impl std::fmt::Display for RetrievalQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        
        if let Some(text) = &self.text {
            parts.push(format!("text: '{}'", text));
        }
        if let Some(domain) = &self.domain {
            parts.push(format!("domain: {:?}", domain));
        }
        if !self.tags.is_empty() {
            parts.push(format!("tags: {:?}", self.tags));
        }
        if let Some(importance) = self.min_importance {
            parts.push(format!("min_importance: {}", importance));
        }
        
        if parts.is_empty() {
            write!(f, "all contexts")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

/// Batch processing for multiple queries
pub struct BatchProcessor {
    processor: Arc<RagProcessor>,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(processor: Arc<RagProcessor>) -> Self {
        Self { processor }
    }

    /// Process multiple queries (sequential for async compatibility)
    pub async fn process_batch(&self, queries: Vec<RetrievalQuery>) -> Vec<ContextResult<RetrievalResult>> {
        let mut results = Vec::with_capacity(queries.len());
        for query in queries {
            results.push(self.processor.retrieve(&query).await);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageConfig;
    use tempfile::TempDir;

    fn create_test_store() -> (Arc<ContextStore>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            persist_path: Some(temp_dir.path().to_path_buf()),
            enable_persistence: true,
            ..Default::default()
        };
        let store = ContextStore::new(config).unwrap();
        (Arc::new(store), temp_dir)
    }

    #[test]
    fn test_retrieval_query() {
        let query = RetrievalQuery::from_text("test query")
            .with_domain(ContextDomain::Code)
            .with_tag("rust");

        assert_eq!(query.text, Some("test query".to_string()));
        assert_eq!(query.domain, Some(ContextDomain::Code));
        assert!(query.tags.contains(&"rust".to_string()));
    }

    #[tokio::test]
    async fn test_rag_processor() {
        let (store, _temp) = create_test_store();
        let processor = RagProcessor::with_defaults(store.clone());

        // Add test context
        let ctx = Context::new("Test content", ContextDomain::Code);
        store.store(ctx).await.unwrap();

        // Retrieve
        let result = processor.retrieve(&RetrievalQuery::new()).await.unwrap();
        assert_eq!(result.candidates_considered, 1);
    }
}
