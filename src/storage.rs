//! Multi-tier storage for context entries
//!
//! Implements a tiered storage system:
//! 1. In-memory LRU cache for hot data
//! 2. Sled embedded database for persistence
//! 3. Optional vector index for similarity search

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::context::{Context, ContextDomain, ContextId, ContextQuery};
use crate::error::{ContextError, Result};

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum items in memory cache
    pub memory_cache_size: usize,
    /// Path for persistent storage (None for in-memory only)
    pub persist_path: Option<PathBuf>,
    /// Enable automatic cleanup of expired contexts
    pub auto_cleanup: bool,
    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,
    /// Enable disk persistence
    pub enable_persistence: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            memory_cache_size: 10_000,
            persist_path: None,
            auto_cleanup: true,
            cleanup_interval_secs: 3600,
            enable_persistence: true,
        }
    }
}

impl StorageConfig {
    /// Create config for in-memory only storage
    pub fn memory_only(cache_size: usize) -> Self {
        Self {
            memory_cache_size: cache_size,
            persist_path: None,
            auto_cleanup: true,
            cleanup_interval_secs: 3600,
            enable_persistence: false,
        }
    }

    /// Create config with disk persistence
    pub fn with_persistence(cache_size: usize, path: impl Into<PathBuf>) -> Self {
        Self {
            memory_cache_size: cache_size,
            persist_path: Some(path.into()),
            auto_cleanup: true,
            cleanup_interval_secs: 3600,
            enable_persistence: true,
        }
    }
}

/// Multi-tier context storage
pub struct ContextStore {
    /// In-memory LRU cache
    memory_cache: Arc<RwLock<LruCache<ContextId, Context>>>,
    /// Persistent storage (sled)
    disk_store: Option<sled::Db>,
    /// Domain index for fast filtering
    domain_index: Arc<RwLock<HashMap<ContextDomain, Vec<ContextId>>>>,
    /// Tag index for fast filtering
    tag_index: Arc<RwLock<HashMap<String, Vec<ContextId>>>>,
    /// Configuration
    config: StorageConfig,
}

impl ContextStore {
    /// Create a new context store
    pub fn new(config: StorageConfig) -> Result<Self> {
        let memory_cache = Arc::new(RwLock::new(LruCache::new(
            std::num::NonZeroUsize::new(config.memory_cache_size)
                .ok_or_else(|| ContextError::Config("Cache size must be > 0".into()))?,
        )));

        let disk_store = if config.enable_persistence {
            let path = config
                .persist_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("./data/context_store"));

            // Ensure directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            Some(sled::open(&path)?)
        } else {
            None
        };

        Ok(Self {
            memory_cache,
            disk_store,
            domain_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Store a context entry
    pub async fn store(&self, context: Context) -> Result<ContextId> {
        let id = context.id.clone();

        // Update indices
        {
            let mut domain_idx = self.domain_index.write().await;
            domain_idx
                .entry(context.domain.clone())
                .or_default()
                .push(id.clone());
        }

        {
            let mut tag_idx = self.tag_index.write().await;
            for tag in &context.metadata.tags {
                tag_idx.entry(tag.clone()).or_default().push(id.clone());
            }
        }

        // Store in memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.put(id.clone(), context.clone());
        }

        // Persist to disk if enabled
        if let Some(ref db) = self.disk_store {
            let serialized = serde_json::to_vec(&context)?;
            db.insert(id.as_str().as_bytes(), serialized)?;
            db.flush_async().await?;
        }

        Ok(id)
    }

    /// Retrieve a context by ID
    pub async fn get(&self, id: &ContextId) -> Result<Option<Context>> {
        // Check memory cache first
        {
            let mut cache = self.memory_cache.write().await;
            if let Some(ctx) = cache.get_mut(id) {
                ctx.mark_accessed();
                return Ok(Some(ctx.clone()));
            }
        }

        // Check disk storage
        if let Some(ref db) = self.disk_store {
            if let Some(data) = db.get(id.as_str().as_bytes())? {
                let mut context: Context = serde_json::from_slice(&data)?;
                context.mark_accessed();

                // Promote to memory cache
                let mut cache = self.memory_cache.write().await;
                cache.put(id.clone(), context.clone());

                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Delete a context by ID
    pub async fn delete(&self, id: &ContextId) -> Result<bool> {
        let mut found = false;

        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            if cache.pop(id).is_some() {
                found = true;
            }
        }

        // Remove from disk
        if let Some(ref db) = self.disk_store {
            if db.remove(id.as_str().as_bytes())?.is_some() {
                found = true;
            }
        }

        // TODO: Clean up indices

        Ok(found)
    }

    /// Query contexts based on criteria
    pub async fn query(&self, query: &ContextQuery) -> Result<Vec<Context>> {
        let mut results = Vec::new();

        // Get candidate IDs from indices
        let candidate_ids = self.get_candidate_ids(query).await;

        // Fetch and filter contexts
        for id in candidate_ids {
            if let Some(ctx) = self.get(&id).await? {
                if self.matches_query(&ctx, query) {
                    results.push(ctx);
                }

                if results.len() >= query.limit {
                    break;
                }
            }
        }

        // Sort by importance and recency
        results.sort_by(|a, b| {
            let importance_cmp = b
                .metadata
                .importance
                .partial_cmp(&a.metadata.importance)
                .unwrap_or(std::cmp::Ordering::Equal);

            if importance_cmp == std::cmp::Ordering::Equal {
                b.accessed_at.cmp(&a.accessed_at)
            } else {
                importance_cmp
            }
        });

        results.truncate(query.limit);
        Ok(results)
    }

    /// Retrieve relevant context for RAG
    pub async fn retrieve_context(
        &self,
        query_text: &str,
        limit: usize,
        domain_filter: Option<&ContextDomain>,
    ) -> Result<Vec<Context>> {
        // Build query
        let mut ctx_query = ContextQuery::new().with_limit(limit);

        if let Some(domain) = domain_filter {
            ctx_query = ctx_query.with_domain(domain.clone());
        }

        // For now, simple text matching
        // TODO: Implement vector similarity when embeddings are available
        let query_lower = query_text.to_lowercase();
        let mut results = Vec::new();

        let cache = self.memory_cache.read().await;
        for (_, ctx) in cache.iter() {
            if ctx.content.to_lowercase().contains(&query_lower) {
                if let Some(domain) = domain_filter {
                    if &ctx.domain != domain {
                        continue;
                    }
                }
                results.push(ctx.clone());
                if results.len() >= limit {
                    break;
                }
            }
        }

        // Sort by importance
        results.sort_by(|a, b| {
            b.metadata
                .importance
                .partial_cmp(&a.metadata.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Get candidate IDs from indices based on query filters
    async fn get_candidate_ids(&self, query: &ContextQuery) -> Vec<ContextId> {
        let mut candidates = Vec::new();

        // If domain filter specified, use domain index
        if let Some(ref domain) = query.domain_filter {
            let domain_idx = self.domain_index.read().await;
            if let Some(ids) = domain_idx.get(domain) {
                candidates.extend(ids.iter().cloned());
            }
        }

        // If tag filter specified, use tag index
        if let Some(ref tags) = query.tag_filter {
            let tag_idx = self.tag_index.read().await;
            for tag in tags {
                if let Some(ids) = tag_idx.get(tag) {
                    candidates.extend(ids.iter().cloned());
                }
            }
        }

        // If no filters, get all from cache
        if candidates.is_empty() && query.domain_filter.is_none() && query.tag_filter.is_none() {
            let cache = self.memory_cache.read().await;
            candidates = cache.iter().map(|(id, _)| id.clone()).collect();
        }

        // Deduplicate
        candidates.sort();
        candidates.dedup();

        candidates
    }

    /// Check if a context matches the query criteria
    fn matches_query(&self, ctx: &Context, query: &ContextQuery) -> bool {
        // Check expiration
        if ctx.is_expired() {
            return false;
        }

        // Check domain
        if let Some(ref domain) = query.domain_filter {
            if &ctx.domain != domain {
                return false;
            }
        }

        // Check source
        if let Some(ref source) = query.source_filter {
            if &ctx.metadata.source != source {
                return false;
            }
        }

        // Check importance
        if let Some(min_importance) = query.min_importance {
            if ctx.metadata.importance < min_importance {
                return false;
            }
        }

        // Check age
        if let Some(max_age) = query.max_age_seconds {
            if ctx.age_seconds() > max_age {
                return false;
            }
        }

        // Check verified status
        if query.verified_only && !ctx.metadata.verified {
            return false;
        }

        // Check text query (simple contains for now)
        if let Some(ref text) = query.query {
            if !ctx.content.to_lowercase().contains(&text.to_lowercase()) {
                return false;
            }
        }

        true
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        let cache = self.memory_cache.read().await;
        let memory_count = cache.len();

        let disk_count = self
            .disk_store
            .as_ref()
            .map(|db| db.len())
            .unwrap_or(0);

        StorageStats {
            memory_count,
            disk_count,
            cache_capacity: self.config.memory_cache_size,
        }
    }

    /// Cleanup expired contexts
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut removed = 0;
        let now = Utc::now();

        // Collect expired IDs
        let expired_ids: Vec<ContextId> = {
            let cache = self.memory_cache.read().await;
            cache
                .iter()
                .filter(|(_, ctx)| ctx.expires_at.map(|exp| now > exp).unwrap_or(false))
                .map(|(id, _)| id.clone())
                .collect()
        };

        // Remove expired contexts
        for id in expired_ids {
            if self.delete(&id).await? {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Number of items in memory cache
    pub memory_count: usize,
    /// Number of items on disk
    pub disk_count: usize,
    /// Memory cache capacity
    pub cache_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let config = StorageConfig::memory_only(100);
        let store = ContextStore::new(config).unwrap();

        let ctx = Context::new("Test content", ContextDomain::Code);
        let id = ctx.id.clone();

        store.store(ctx).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test content");
    }

    #[tokio::test]
    async fn test_query_by_domain() {
        let config = StorageConfig::memory_only(100);
        let store = ContextStore::new(config).unwrap();

        let ctx1 = Context::new("Code content", ContextDomain::Code);
        let ctx2 = Context::new("Doc content", ContextDomain::Documentation);

        store.store(ctx1).await.unwrap();
        store.store(ctx2).await.unwrap();

        let query = ContextQuery::new().with_domain(ContextDomain::Code);
        let results = store.query(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].domain, ContextDomain::Code);
    }
}
