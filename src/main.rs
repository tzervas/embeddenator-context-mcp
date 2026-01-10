//! context-mcp
//!
//! MCP server for context management with temporal reasoning and RAG support.
//!
//! This crate provides a Model Context Protocol (MCP) server for storing,
//! retrieving, and querying context with:
//! - Multi-tier storage (LRU memory cache + sled disk persistence)
//! - Temporal reasoning with time-based filtering and decay scoring
//! - CPU-optimized RAG processing with parallel execution
//! - Security screening status integration
//!
//! # Usage
//!
//! Run as HTTP server:
//! ```bash
//! context-mcp --host 127.0.0.1 --port 3000
//! ```
//!
//! Run as stdio transport:
//! ```bash
//! context-mcp --stdio
//! ```

use clap::Parser;
use std::path::PathBuf;

use context_mcp::{
    rag::RagConfig,
    server::{McpServer, ServerConfig, StdioTransport},
    storage::StorageConfig,
};

/// MCP Context Management Server
#[derive(Parser, Debug)]
#[command(name = "context-mcp")]
#[command(about = "Context management MCP server with temporal reasoning")]
#[command(version)]
struct Args {
    /// Use stdio transport instead of HTTP
    #[arg(long)]
    stdio: bool,

    /// Server host (HTTP mode only)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port (HTTP mode only)
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Path for persistent storage
    #[arg(long)]
    storage_path: Option<PathBuf>,

    /// Memory cache size
    #[arg(long, default_value = "1000")]
    cache_size: usize,

    /// Enable disk persistence
    #[arg(long)]
    persist: bool,

    /// Number of RAG threads (0 = auto)
    #[arg(long, default_value = "0")]
    threads: usize,

    /// Disable temporal decay scoring
    #[arg(long)]
    no_decay: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    // Build configuration
    let storage_config = StorageConfig {
        memory_cache_size: args.cache_size,
        persist_path: args.storage_path,
        enable_persistence: args.persist,
        auto_cleanup: true,
        cleanup_interval_secs: 300,
    };

    let rag_config = RagConfig {
        num_threads: args.threads,
        temporal_decay: !args.no_decay,
        ..Default::default()
    };

    let server_config = ServerConfig {
        host: args.host,
        port: args.port,
        storage: storage_config,
        rag: rag_config,
    };

    if args.stdio {
        tracing::info!("Starting MCP Context Server in stdio mode");
        let transport = StdioTransport::new(server_config)?;
        transport.run().await?;
    } else {
        tracing::info!(
            "Starting MCP Context Server on {}:{}",
            server_config.host,
            server_config.port
        );
        let server = McpServer::new(server_config)?;
        server.run().await?;
    }

    Ok(())
}
