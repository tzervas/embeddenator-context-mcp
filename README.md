# context-mcp

[![Crates.io](https://img.shields.io/crates/v/context-mcp.svg)](https://crates.io/crates/context-mcp)
[![Documentation](https://docs.rs/context-mcp/badge.svg)](https://docs.rs/context-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

MCP server for context management, temporal metadata, and lightweight retrieval/RAG plumbing.

This crate is intended to be a "memory service" for agents: store/retrieve context items with timestamps and metadata, and support simple query/retrieval patterns.

## Features

- **Multi-tier Storage**: In-memory (LRU), cache, and disk persistence
- **Temporal Reasoning**: Timestamps and age tracking for context relevance
- **RAG Processing**: CPU-optimized retrieval-augmented generation support
- **MCP Protocol**: Full compatibility with VS Code, Copilot, and other MCP clients
- **Safe Input Handling**: Integrates with security-mcp for screened inputs

## Status

Alpha / under active development. APIs and tool names may change.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
context-mcp = "0.1.0-alpha.1"
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

## What It Does Today (Code-Backed)

- Runs an MCP-compatible JSON-RPC server over HTTP/WebSocket.
- Stores context with IDs, domains, timestamps, and metadata.
- Supports tiered storage primitives (in-memory + on-disk via `sled`).
- Provides MCP tools for context storage, retrieval, and querying.
- Basic temporal reasoning with time-based filtering.

## What It Does Not Do Yet

- Production-grade embedding/vector search.
- Full RAG pipelines with reliable chunking/citation policies.
- Strong consistency guarantees across distributed replicas.
- Advanced query capabilities (semantic search, etc.).

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   MCP Client    │    │  Context Gateway │    │ Storage Layer   │
│                 │    │                  │    │                 │
│ • VS Code       │◄──►│ • Store/Retrieve │◄──►│ • In-Memory LRU │
│ • Copilot       │    │ • Temporal Query │    │ • Sled Disk DB  │
│ • CLI Tools     │    │ • RAG Processing │    │ • Vector Index  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and contribution guidelines.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
