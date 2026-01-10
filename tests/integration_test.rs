//! Integration tests for storage index cleanup and consistency

use context_mcp::{Context, ContextId, ContextStore, StorageConfig};
use context_mcp::context::{ContextDomain, ContextQuery};

#[tokio::test]
async fn test_delete_cleans_domain_index() {
    let config = StorageConfig {
        memory_cache_size: 100,
        enable_persistence: false,
        ..Default::default()
    };
    let store = ContextStore::new(config).unwrap();
    
    // Store a context
    let ctx = Context::new("Test content".to_string(), ContextDomain::Code);
    let id = ctx.id.clone();
    store.store(ctx).await.unwrap();
    
    // Verify it's in the domain index by querying
    let query = ContextQuery {
        domain_filter: Some(ContextDomain::Code),
        limit: 10,
        ..Default::default()
    };
    let results = store.query(&query).await.unwrap();
    assert_eq!(results.len(), 1);
    
    // Delete the context
    let deleted = store.delete(&id).await.unwrap();
    assert!(deleted);
    
    // Verify it's removed from domain index
    let results_after = store.query(&query).await.unwrap();
    assert_eq!(results_after.len(), 0, "Context should be removed from domain index");
}

#[tokio::test]
async fn test_delete_cleans_tag_index() {
    let config = StorageConfig {
        memory_cache_size: 100,
        enable_persistence: false,
        ..Default::default()
    };
    let store = ContextStore::new(config).unwrap();
    
    // Store a context with tags
    let mut ctx = Context::new("Test content".to_string(), ContextDomain::General);
    ctx.metadata.tags = vec!["test".to_string(), "rust".to_string()];
    let id = ctx.id.clone();
    store.store(ctx).await.unwrap();
    
    // Query by tag
    let query = ContextQuery {
        tag_filter: Some(vec!["test".to_string()]),
        limit: 10,
        ..Default::default()
    };
    let results = store.query(&query).await.unwrap();
    assert_eq!(results.len(), 1);
    
    // Delete the context
    store.delete(&id).await.unwrap();
    
    // Verify it's removed from tag index
    let results_after = store.query(&query).await.unwrap();
    assert_eq!(results_after.len(), 0, "Context should be removed from tag index");
}

#[tokio::test]
async fn test_delete_nonexistent_context() {
    let config = StorageConfig {
        memory_cache_size: 100,
        enable_persistence: false,
        ..Default::default()
    };
    let store = ContextStore::new(config).unwrap();
    
    let fake_id = ContextId::from_string("nonexistent".to_string());
    let deleted = store.delete(&fake_id).await.unwrap();
    assert!(!deleted, "Should return false for nonexistent context");
}

#[tokio::test]
async fn test_storage_consistency_after_multiple_deletes() {
    let config = StorageConfig {
        memory_cache_size: 100,
        enable_persistence: false,
        ..Default::default()
    };
    let store = ContextStore::new(config).unwrap();
    
    // Store multiple contexts with same domain
    let mut ids = Vec::new();
    for i in 0..10 {
        let ctx = Context::new(format!("Content {}", i), ContextDomain::Code);
        ids.push(ctx.id.clone());
        store.store(ctx).await.unwrap();
    }
    
    // Delete half of them
    for id in ids.iter().take(5) {
        store.delete(id).await.unwrap();
    }
    
    // Query should only return remaining contexts
    let query = ContextQuery {
        domain_filter: Some(ContextDomain::Code),
        limit: 10,
        ..Default::default()
    };
    let results = store.query(&query).await.unwrap();
    assert_eq!(results.len(), 5, "Should have 5 remaining contexts");
}
