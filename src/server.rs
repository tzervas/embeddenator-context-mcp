//! MCP server implementation using Axum
//!
//! Provides HTTP/SSE transport for the context management MCP server.

use axum::{
    extract::{Json, State},
    response::{IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use futures::stream::{self, Stream};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;

use crate::error::ContextResult;
use crate::protocol::{
    CallToolRequest, InitializeResult, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse, MCP_VERSION, RequestId, ServerCapabilities, ServerInfo,
    ToolsCapability,
};
use crate::rag::{RagConfig, RagProcessor};
use crate::storage::{ContextStore, StorageConfig};
use crate::tools::ToolRegistry;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Storage configuration
    pub storage: StorageConfig,
    /// RAG configuration
    pub rag: RagConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            storage: StorageConfig::default(),
            rag: RagConfig::default(),
        }
    }
}

/// Shared server state
#[allow(dead_code)]
pub struct ServerState {
    store: Arc<ContextStore>,
    rag: Arc<RagProcessor>,
    tools: Arc<ToolRegistry>,
}

impl ServerState {
    /// Create new server state
    pub fn new(config: &ServerConfig) -> ContextResult<Self> {
        let store = Arc::new(ContextStore::new(config.storage.clone())?);
        let rag = Arc::new(RagProcessor::new(store.clone(), config.rag.clone()));
        let tools = Arc::new(ToolRegistry::new(store.clone(), rag.clone()));

        Ok(Self { store, rag, tools })
    }
}

/// MCP Server
pub struct McpServer {
    config: ServerConfig,
    state: Arc<ServerState>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: ServerConfig) -> ContextResult<Self> {
        let state = Arc::new(ServerState::new(&config)?);
        Ok(Self { config, state })
    }

    /// Create with default configuration
    pub fn with_defaults() -> ContextResult<Self> {
        Self::new(ServerConfig::default())
    }

    /// Build the router
    pub fn router(&self) -> Router {
        Router::new()
            .route("/", get(health))
            .route("/health", get(health))
            .route("/mcp", post(handle_mcp_request))
            .route("/sse", get(sse_handler))
            .with_state(self.state.clone())
    }

    /// Run the server
    pub async fn run(&self) -> ContextResult<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| crate::error::ContextError::Io(e))?;

        tracing::info!("MCP Context Server listening on {}", addr);

        axum::serve(listener, self.router())
            .await
            .map_err(|e| crate::error::ContextError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Get server address
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }
}

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "server": "context-mcp",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Handle MCP JSON-RPC request
async fn handle_mcp_request(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let response = process_request(&state, request).await;
    Json(response)
}

/// Process a single MCP request
async fn process_request(state: &ServerState, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "initialized" => handle_initialized(request.id),
        "tools/list" => handle_list_tools(request.id, state),
        "tools/call" => handle_call_tool(request.id, state, request.params).await,
        "ping" => handle_ping(request.id),
        method => JsonRpcResponse::error(
            request.id,
            JsonRpcError::method_not_found(method),
        ),
    }
}

/// Handle initialize request
fn handle_initialize(id: RequestId) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: MCP_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: true }),
            resources: None,
            prompts: None,
        },
        server_info: ServerInfo {
            name: "context-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}

/// Handle initialized notification
fn handle_initialized(id: RequestId) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({}))
}

/// Handle tools/list request
fn handle_list_tools(id: RequestId, state: &ServerState) -> JsonRpcResponse {
    let tools = state.tools.list_tools();
    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

/// Handle tools/call request
async fn handle_call_tool(
    id: RequestId,
    state: &ServerState,
    params: Option<Value>,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing params"))
        }
    };

    let call_request: CallToolRequest = match serde_json::from_value(params) {
        Ok(r) => r,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
            )
        }
    };

    let result = state.tools.execute(&call_request.name, call_request.arguments).await;
    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}

/// Handle ping request
fn handle_ping(id: RequestId) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({}))
}

/// SSE handler for streaming updates
async fn sse_handler(
    State(_state): State<Arc<ServerState>>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let stream = stream::iter(vec![
        Ok(axum::response::sse::Event::default()
            .event("connected")
            .data("MCP Context Server connected")),
    ]);

    Sse::new(stream)
}

/// Stdio transport for MCP
pub struct StdioTransport {
    state: Arc<ServerState>,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new(config: ServerConfig) -> ContextResult<Self> {
        let state = Arc::new(ServerState::new(&config)?);
        Ok(Self { state })
    }

    /// Run the stdio transport
    pub async fn run(&self) -> ContextResult<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<JsonRpcRequest>(line) {
                        Ok(request) => {
                            let response = process_request(&self.state, request).await;
                            let response_str = serde_json::to_string(&response).unwrap();
                            stdout.write_all(response_str.as_bytes()).await.ok();
                            stdout.write_all(b"\n").await.ok();
                            stdout.flush().await.ok();
                        }
                        Err(_e) => {
                            let error = JsonRpcResponse::error(
                                RequestId::Number(0),
                                JsonRpcError::parse_error(),
                            );
                            let error_str = serde_json::to_string(&error).unwrap();
                            stdout.write_all(error_str.as_bytes()).await.ok();
                            stdout.write_all(b"\n").await.ok();
                            stdout.flush().await.ok();
                        }
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        // Basic test that it responds
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
    }
}
