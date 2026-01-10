# context-mcp

[![Crates.io](https://img.shields.io/crates/v/context-mcp.svg)](https://crates.io/crates/context-mcp)
[![Documentation](https://docs.rs/context-mcp/badge.svg)](https://docs.rs/context-mcp/latest/context_mcp/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

MCP server for context management, temporal metadata, and lightweight retrieval plumbing.

This crate provides a "memory service" for agents: store/retrieve context items with timestamps and metadata, supporting basic query/retrieval patterns via text matching and filtering.

## Features

- **Multi-tier Storage**: In-memory LRU cache with optional sled-based disk persistence
- **Temporal Tracking**: Timestamps, age tracking, and time-based filtering for context relevance
- **CPU-Optimized Retrieval**: Parallel processing with rayon for text-based context queries
- **MCP Protocol Support**: JSON-RPC server implementation with HTTP/WebSocket and stdio transports
- **Screening Status Fields**: Built-in fields for tracking security screening state (integration not included)

## Status

Alpha / under active development. APIs and tool names may change.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
context-mcp = "0.1"
```

## Usage

### As a Library

```rust
use context_mcp::{ContextStore, StorageConfig, Context, ContextDomain};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage configuration
    let config = StorageConfig {
        memory_cache_size: 1000,
        enable_persistence: true,
    };

    // Create context store
    let store = ContextStore::new(config)?;

    // Store some context
    let ctx = Context::new("This is some important information", ContextDomain::Code);
    let id = store.store(ctx).await?;

    // Retrieve it
    let retrieved = store.get(&id).await?;
    println!("Retrieved: {}", retrieved.content);

    Ok(())
}
```

### As an MCP Server

Run as HTTP server:
```bash
context-mcp --host 127.0.0.1 --port 3000
```

Run as stdio transport:
```bash
context-mcp --stdio
```

## What It Does (Verified by Code/Tests)

- **JSON-RPC MCP Server**: Runs over HTTP/WebSocket or stdio transport
- **Context Storage**: Store/retrieve contexts with IDs, domains, timestamps, tags, and custom metadata
- **Tiered Storage**: In-memory LRU cache (always) + optional sled disk persistence
- **Text-Based Queries**: Query by text content, domain, tags, time ranges with simple text matching
- **Temporal Filtering**: Filter contexts by creation time, last access, age, and expiration
- **Parallel Processing**: CPU-optimized retrieval using rayon for performance
- **MCP Tools**: 10 tools including store_context, get_context, query_contexts, retrieve_contexts, delete_context, update_screening, get_temporal_stats, get_storage_stats, cleanup_expired

## What It Does Not Do (Yet)

- **Vector embeddings**: Mock implementation only - no real embedding generation or similarity search
- **Semantic search**: Text matching is literal, not semantic
- **External integrations**: No active security-mcp or other service integrations (only status fields)
- **Chunking/citations**: No automatic document chunking or citation tracking
- **Distributed storage**: Single-node only, no replication or clustering

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   MCP Client    │    │  JSON-RPC Server │    │ Storage Layer   │
│                 │    │                  │    │                 │
│ • HTTP/WS       │◄──►│ • Store/Retrieve │◄──►│ • In-Memory LRU │
│ • stdio         │    │ • Query/Filter   │    │ • Sled (opt)    │
│ • curl/tools    │    │ • Temporal Stats │    │ • Domain Index  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────┐
                       │ RAG Processor│
                       │ • Text Match │
                       │ • Parallel   │
                       │ • Scoring    │
                       └──────────────┘
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and contribution guidelines.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
