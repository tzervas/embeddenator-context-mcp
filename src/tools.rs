//! MCP tool implementations for context management
//!
//! Provides tools for storing, retrieving, and querying contexts
//! with temporal reasoning and RAG support.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{Context, ContextDomain, ContextMetadata, ContextQuery, ScreeningStatus};
use crate::error::ContextResult;
use crate::protocol::{
    CallToolResult, InputSchema, PropertySchema, Tool,
};
use crate::rag::{RagProcessor, RetrievalQuery};
use crate::storage::ContextStore;
use crate::temporal::TemporalQuery;

/// Tool registry managing all available tools
pub struct ToolRegistry {
    store: Arc<ContextStore>,
    rag: Arc<RagProcessor>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new(store: Arc<ContextStore>, rag: Arc<RagProcessor>) -> Self {
        Self { store, rag }
    }

    /// Get all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            self.store_context_tool(),
            self.get_context_tool(),
            self.delete_context_tool(),
            self.query_contexts_tool(),
            self.retrieve_contexts_tool(),
            self.update_screening_tool(),
            self.get_temporal_stats_tool(),
            self.get_storage_stats_tool(),
            self.cleanup_expired_tool(),
        ]
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: HashMap<String, Value>) -> CallToolResult {
        match name {
            "store_context" => self.store_context(args).await,
            "get_context" => self.get_context(args).await,
            "delete_context" => self.delete_context(args).await,
            "query_contexts" => self.query_contexts(args).await,
            "retrieve_contexts" => self.retrieve_contexts(args).await,
            "update_screening" => self.update_screening(args).await,
            "get_temporal_stats" => self.get_temporal_stats(args).await,
            "get_storage_stats" => self.get_storage_stats(args).await,
            "cleanup_expired" => self.cleanup_expired(args).await,
            _ => CallToolResult::error(format!("Unknown tool: {}", name)),
        }
    }

    // Tool definitions

    fn store_context_tool(&self) -> Tool {
        Tool {
            name: "store_context".to_string(),
            description: Some("Store a new context with metadata and optional TTL".to_string()),
            input_schema: InputSchema::object()
                .with_required("content", PropertySchema::string("The context content"))
                .with_property(
                    "domain",
                    PropertySchema::string("Context domain")
                        .with_enum(vec![
                            "General", "Code", "Documentation", "Conversation",
                            "Filesystem", "WebSearch", "Dataset", "Research",
                        ]),
                )
                .with_property("source", PropertySchema::string("Source of the context"))
                .with_property("tags", PropertySchema::array("Tags for categorization"))
                .with_property(
                    "importance",
                    PropertySchema::number("Importance 0.0-1.0").with_default(json!(0.5)),
                )
                .with_property(
                    "ttl_hours",
                    PropertySchema::number("Time to live in hours"),
                ),
        }
    }

    fn get_context_tool(&self) -> Tool {
        Tool {
            name: "get_context".to_string(),
            description: Some("Retrieve a context by ID".to_string()),
            input_schema: InputSchema::object()
                .with_required("id", PropertySchema::string("Context ID")),
        }
    }

    fn delete_context_tool(&self) -> Tool {
        Tool {
            name: "delete_context".to_string(),
            description: Some("Delete a context by ID".to_string()),
            input_schema: InputSchema::object()
                .with_required("id", PropertySchema::string("Context ID")),
        }
    }

    fn query_contexts_tool(&self) -> Tool {
        Tool {
            name: "query_contexts".to_string(),
            description: Some("Query contexts with filters".to_string()),
            input_schema: InputSchema::object()
                .with_property("domain", PropertySchema::string("Filter by domain"))
                .with_property("tags", PropertySchema::array("Filter by tags"))
                .with_property(
                    "min_importance",
                    PropertySchema::number("Minimum importance threshold"),
                )
                .with_property(
                    "max_age_hours",
                    PropertySchema::number("Maximum age in hours"),
                )
                .with_property(
                    "verified_only",
                    PropertySchema::boolean("Only return verified contexts"),
                )
                .with_property(
                    "limit",
                    PropertySchema::number("Maximum results").with_default(json!(10)),
                ),
        }
    }

    fn retrieve_contexts_tool(&self) -> Tool {
        Tool {
            name: "retrieve_contexts".to_string(),
            description: Some("Retrieve contexts using RAG with scoring".to_string()),
            input_schema: InputSchema::object()
                .with_property("text", PropertySchema::string("Text query"))
                .with_property("domain", PropertySchema::string("Domain filter"))
                .with_property("tags", PropertySchema::array("Tag filters"))
                .with_property(
                    "min_importance",
                    PropertySchema::number("Minimum importance"),
                )
                .with_property(
                    "max_age_hours",
                    PropertySchema::number("Maximum age for temporal filtering"),
                )
                .with_property(
                    "max_results",
                    PropertySchema::number("Maximum results").with_default(json!(10)),
                ),
        }
    }

    fn update_screening_tool(&self) -> Tool {
        Tool {
            name: "update_screening".to_string(),
            description: Some("Update screening status of a context".to_string()),
            input_schema: InputSchema::object()
                .with_required("id", PropertySchema::string("Context ID"))
                .with_required(
                    "status",
                    PropertySchema::string("New screening status")
                        .with_enum(vec!["Safe", "Flagged", "Blocked"]),
                )
                .with_property("reason", PropertySchema::string("Reason for status change")),
        }
    }

    fn get_temporal_stats_tool(&self) -> Tool {
        Tool {
            name: "get_temporal_stats".to_string(),
            description: Some("Get temporal statistics for stored contexts".to_string()),
            input_schema: InputSchema::object()
                .with_property("domain", PropertySchema::string("Filter by domain")),
        }
    }

    fn get_storage_stats_tool(&self) -> Tool {
        Tool {
            name: "get_storage_stats".to_string(),
            description: Some("Get storage statistics".to_string()),
            input_schema: InputSchema::object(),
        }
    }

    fn cleanup_expired_tool(&self) -> Tool {
        Tool {
            name: "cleanup_expired".to_string(),
            description: Some("Remove expired contexts".to_string()),
            input_schema: InputSchema::object(),
        }
    }

    // Tool implementations

    async fn store_context(&self, args: HashMap<String, Value>) -> CallToolResult {
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return CallToolResult::error("Missing required parameter: content"),
        };

        let domain = args
            .get("domain")
            .and_then(|v| v.as_str())
            .map(parse_domain)
            .unwrap_or(ContextDomain::General);

        let mut ctx = Context::new(content, domain);

        // Set metadata
        if let Some(source) = args.get("source").and_then(|v| v.as_str()) {
            ctx.metadata.source = source.to_string();
        }

        if let Some(tags) = args.get("tags").and_then(|v| v.as_array()) {
            ctx.metadata.tags = tags
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(importance) = args.get("importance").and_then(|v| v.as_f64()) {
            ctx.metadata.importance = importance.clamp(0.0, 1.0) as f32;
        }

        if let Some(ttl) = args.get("ttl_hours").and_then(|v| v.as_i64()) {
            ctx = ctx.with_ttl(std::time::Duration::from_secs(ttl as u64 * 3600));
        }

        let id = ctx.id.clone();
        match self.store.store(ctx).await {
            Ok(_stored_id) => CallToolResult::json(json!({
                "success": true,
                "id": id.to_string(),
                "message": "Context stored successfully"
            })),
            Err(e) => CallToolResult::error(format!("Failed to store context: {}", e)),
        }
    }

    async fn get_context(&self, args: HashMap<String, Value>) -> CallToolResult {
        let id_str = match args.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return CallToolResult::error("Missing required parameter: id"),
        };

        let id = crate::context::ContextId::from_string(id_str.to_string());
        
        match self.store.get(&id).await {
            Ok(Some(ctx)) => CallToolResult::json(json!({
                "id": ctx.id.to_string(),
                "content": ctx.content,
                "domain": format!("{:?}", ctx.domain),
                "created_at": ctx.created_at.to_rfc3339(),
                "accessed_at": ctx.accessed_at.to_rfc3339(),
                "metadata": {
                    "source": ctx.metadata.source,
                    "tags": ctx.metadata.tags,
                    "importance": ctx.metadata.importance,
                    "verified": ctx.metadata.verified,
                    "screening_status": format!("{:?}", ctx.metadata.screening_status)
                },
                "age_hours": ctx.age_hours()
            })),
            Ok(None) => CallToolResult::error(format!("Context not found: {}", id_str)),
            Err(e) => CallToolResult::error(format!("Error retrieving context: {}", e)),
        }
    }

    async fn delete_context(&self, args: HashMap<String, Value>) -> CallToolResult {
        let id_str = match args.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return CallToolResult::error("Missing required parameter: id"),
        };

        let id = crate::context::ContextId::from_string(id_str.to_string());
        
        match self.store.delete(&id).await {
            Ok(true) => CallToolResult::json(json!({
                "success": true,
                "message": "Context deleted"
            })),
            Ok(false) => CallToolResult::error(format!("Context not found: {}", id_str)),
            Err(e) => CallToolResult::error(format!("Error deleting context: {}", e)),
        }
    }

    async fn query_contexts(&self, args: HashMap<String, Value>) -> CallToolResult {
        let mut query = ContextQuery::new();

        if let Some(domain) = args.get("domain").and_then(|v| v.as_str()) {
            query = query.with_domain(parse_domain(domain));
        }

        if let Some(tags) = args.get("tags").and_then(|v| v.as_array()) {
            for tag in tags.iter().filter_map(|v| v.as_str()) {
                query = query.with_tag(tag.to_string());
            }
        }

        if let Some(min_importance) = args.get("min_importance").and_then(|v| v.as_f64()) {
            query = query.with_min_importance(min_importance as f32);
        }

        if let Some(max_age) = args.get("max_age_hours").and_then(|v| v.as_i64()) {
            query = query.with_max_age_hours(max_age);
        }

        if let Some(verified) = args.get("verified_only").and_then(|v| v.as_bool()) {
            if verified {
                query = query.verified_only();
            }
        }

        if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
            query = query.with_limit(limit as usize);
        }

        match self.store.query(&query).await {
            Ok(contexts) => {
                let results: Vec<Value> = contexts
                    .iter()
                    .map(|ctx| {
                        json!({
                            "id": ctx.id.to_string(),
                            "content_preview": ctx.content.chars().take(100).collect::<String>(),
                            "domain": format!("{:?}", ctx.domain),
                            "importance": ctx.metadata.importance,
                            "age_hours": ctx.age_hours(),
                            "tags": ctx.metadata.tags
                        })
                    })
                    .collect();

                CallToolResult::json(json!({
                    "count": results.len(),
                    "contexts": results
                }))
            }
            Err(e) => CallToolResult::error(format!("Query failed: {}", e)),
        }
    }

    async fn retrieve_contexts(&self, args: HashMap<String, Value>) -> CallToolResult {
        let mut query = RetrievalQuery::new();

        if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
            query.text = Some(text.to_string());
        }

        if let Some(domain) = args.get("domain").and_then(|v| v.as_str()) {
            query = query.with_domain(parse_domain(domain));
        }

        if let Some(tags) = args.get("tags").and_then(|v| v.as_array()) {
            for tag in tags.iter().filter_map(|v| v.as_str()) {
                query = query.with_tag(tag.to_string());
            }
        }

        if let Some(min_importance) = args.get("min_importance").and_then(|v| v.as_f64()) {
            query = query.with_min_importance(min_importance as f32);
        }

        if let Some(max_age) = args.get("max_age_hours").and_then(|v| v.as_i64()) {
            query = query.with_temporal(TemporalQuery::recent(max_age));
        }

        match self.rag.retrieve(&query).await {
            Ok(result) => {
                let contexts: Vec<Value> = result
                    .contexts
                    .iter()
                    .map(|sc| {
                        json!({
                            "id": sc.context.id.to_string(),
                            "content": sc.context.content,
                            "domain": format!("{:?}", sc.context.domain),
                            "score": sc.score,
                            "score_breakdown": {
                                "temporal": sc.score_breakdown.temporal,
                                "importance": sc.score_breakdown.importance,
                                "domain_match": sc.score_breakdown.domain_match,
                                "tag_match": sc.score_breakdown.tag_match
                            },
                            "age_hours": sc.context.age_hours(),
                            "tags": sc.context.metadata.tags
                        })
                    })
                    .collect();

                CallToolResult::json(json!({
                    "count": contexts.len(),
                    "candidates_considered": result.candidates_considered,
                    "processing_time_ms": result.processing_time_ms,
                    "temporal_stats": {
                        "count": result.temporal_stats.count,
                        "avg_age_hours": result.temporal_stats.avg_age_hours,
                        "distribution": result.temporal_stats.distribution
                    },
                    "contexts": contexts
                }))
            }
            Err(e) => CallToolResult::error(format!("Retrieval failed: {}", e)),
        }
    }

    async fn update_screening(&self, args: HashMap<String, Value>) -> CallToolResult {
        let id_str = match args.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return CallToolResult::error("Missing required parameter: id"),
        };

        let status_str = match args.get("status").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return CallToolResult::error("Missing required parameter: status"),
        };

        let status = match status_str.to_lowercase().as_str() {
            "safe" => ScreeningStatus::Safe,
            "flagged" => ScreeningStatus::Flagged,
            "blocked" => ScreeningStatus::Blocked,
            _ => return CallToolResult::error(format!("Invalid status: {}", status_str)),
        };

        let id = crate::context::ContextId::from_string(id_str.to_string());
        
        match self.store.get(&id).await {
            Ok(Some(mut ctx)) => {
                ctx.metadata.screening_status = status.clone();
                match self.store.store(ctx).await {
                    Ok(_) => CallToolResult::json(json!({
                        "success": true,
                        "id": id_str,
                        "new_status": format!("{:?}", status)
                    })),
                    Err(e) => CallToolResult::error(format!("Failed to update: {}", e)),
                }
            }
            Ok(None) => CallToolResult::error(format!("Context not found: {}", id_str)),
            Err(e) => CallToolResult::error(format!("Error: {}", e)),
        }
    }

    async fn get_temporal_stats(&self, args: HashMap<String, Value>) -> CallToolResult {
        let mut query = ContextQuery::new();

        if let Some(domain) = args.get("domain").and_then(|v| v.as_str()) {
            query = query.with_domain(parse_domain(domain));
        }

        match self.store.query(&query).await {
            Ok(contexts) => {
                let stats = crate::temporal::TemporalStats::from_contexts(&contexts);
                CallToolResult::json(json!({
                    "count": stats.count,
                    "oldest": stats.oldest.map(|t| t.to_rfc3339()),
                    "newest": stats.newest.map(|t| t.to_rfc3339()),
                    "avg_age_hours": stats.avg_age_hours,
                    "distribution": {
                        "last_hour": stats.distribution.last_hour,
                        "last_day": stats.distribution.last_day,
                        "last_week": stats.distribution.last_week,
                        "last_month": stats.distribution.last_month,
                        "older": stats.distribution.older
                    }
                }))
            }
            Err(e) => CallToolResult::error(format!("Failed to get stats: {}", e)),
        }
    }

    async fn get_storage_stats(&self, _args: HashMap<String, Value>) -> CallToolResult {
        let stats = self.store.stats().await;
        CallToolResult::json(json!({
            "memory_count": stats.memory_count,
            "disk_count": stats.disk_count,
            "cache_capacity": stats.cache_capacity
        }))
    }

    async fn cleanup_expired(&self, _args: HashMap<String, Value>) -> CallToolResult {
        match self.store.cleanup_expired().await {
            Ok(count) => CallToolResult::json(json!({
                "success": true,
                "removed_count": count
            })),
            Err(e) => CallToolResult::error(format!("Cleanup failed: {}", e)),
        }
    }
}

/// Parse domain string to enum
fn parse_domain(s: &str) -> ContextDomain {
    match s.to_lowercase().as_str() {
        "code" => ContextDomain::Code,
        "documentation" | "docs" => ContextDomain::Documentation,
        "conversation" | "chat" => ContextDomain::Conversation,
        "filesystem" | "files" => ContextDomain::Filesystem,
        "websearch" | "web" => ContextDomain::WebSearch,
        "dataset" | "data" => ContextDomain::Dataset,
        "research" => ContextDomain::Research,
        _ => ContextDomain::General,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_domain() {
        assert_eq!(parse_domain("Code"), ContextDomain::Code);
        assert_eq!(parse_domain("docs"), ContextDomain::Documentation);
        assert_eq!(parse_domain("unknown"), ContextDomain::General);
    }
}
