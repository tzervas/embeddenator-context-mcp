# Embeddenator Context MCP - Ecosystem Context Report

**Generated**: 2026-01-04  
**Purpose**: Cross-project context for AI-assisted development

## Project Identity

| Field | Value |
|-------|-------|
| **Name** | embeddenator-context-mcp |
| **Type** | Native MCP Server (Rust binary) |
| **Transport** | stdio / HTTP |
| **Role** | Context management with temporal reasoning and RAG |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    MCP Client                                    │
└───────────────────────────┬─────────────────────────────────────┘
                            │ MCP Protocol
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                   embeddenator-context-mcp                       │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐    │
│  │  LRU Memory    │  │  Sled Disk     │  │  Temporal      │    │
│  │  Cache         │  │  Persistence   │  │  Reasoning     │    │
│  └────────────────┘  └────────────────┘  └────────────────┘    │
│  ┌────────────────────────────────────────────────────────┐    │
│  │                   RAG Processor                         │    │
│  │  (CPU-optimized, parallel execution)                    │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## Capabilities

- **Multi-tier Storage**: LRU memory cache + sled disk persistence
- **Temporal Reasoning**: Time-based filtering and decay scoring
- **RAG Processing**: CPU-optimized with parallel execution
- **Security Integration**: Tracks screening status of stored content

## Sister Projects

### Same Ecosystem (Native MCP Servers)

| Project | Relationship | Integration Point |
|---------|--------------|-------------------|
| **embeddenator-agent-mcp** | Consumer | Stores workflow state between steps |
| **embeddenator-webpuppet-mcp** | Sibling | Can store conversation transcripts |
| **embeddenator-security-mcp** | Upstream | Screens content before storage |

### WASM Counterpart (homelab-ci-stack)

| Project | Comparison | Notes |
|---------|------------|-------|
| **homelab-ci-stack/context-mcp** | Lightweight alternative | WASM-based, ~160KB, no persistence |

#### Feature Comparison

| Feature | embeddenator-context-mcp | homelab-ci-stack/context-mcp |
|---------|--------------------------|------------------------------|
| **Execution** | Native binary | WASM (wasmtime) |
| **Persistence** | Yes (sled) | No (stateless) |
| **Temporal Reasoning** | Yes | No |
| **RAG Processing** | Yes (parallel) | No |
| **Memory Cache** | Yes (LRU) | No |
| **Startup Time** | ~100ms | ~18ms |
| **Use Case** | Long-running workflows | One-shot text processing |

## Integration Patterns

### 1. Agent Workflow Context

```
agent-mcp.workflow_start()
    │
    ├─► Step 1 completes
    │   └─► context-mcp.store(step1_result, temporal_weight=high)
    │
    ├─► Step 2 starts
    │   ├─► context-mcp.retrieve(filter=recent)
    │   └─► context-mcp.store(step2_result)
    │
    └─► Final: context-mcp.summarize(workflow_id)
```

### 2. Security-Aware Storage

```
input → security-mcp.screen()
           │
           ├─► if clean: context-mcp.store(input, screening_status=passed)
           │
           └─► if flagged: context-mcp.store(input, screening_status=flagged, redacted=true)
```

### 3. Hybrid WASM + Native

```
# Fast path (WASM) - stateless processing
echo '{"method":"summarize","params":{"text":"..."}}' | wasmtime context-mcp.wasm

# Full path (Native) - persistent storage + RAG
context-mcp --stdio
  → store(key, value, ttl)
  → retrieve(key, temporal_filter)
  → query_similar(embedding, top_k)
```

## Configuration

```bash
# Full featured (HTTP + persistence)
context-mcp --host 127.0.0.1 --port 3000 --persist --storage-path /data/context

# Lightweight (stdio, memory only)
context-mcp --stdio --cache-size 500 --no-decay

# RAG optimized
context-mcp --stdio --threads 4 --persist
```

## Key Dependencies

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
sled = "0.34"           # Embedded database
lru = "0.12"            # Memory cache
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
rayon = "1.8"           # Parallel RAG processing
```

## Source Structure

```
src/
├── main.rs      # CLI entry point
├── lib.rs       # Public API exports
├── server.rs    # MCP server + StdioTransport
├── storage.rs   # StorageConfig, multi-tier storage
├── rag.rs       # RagConfig, parallel processing
└── temporal.rs  # Time-based scoring and decay
```

## Testing & Validation

### Local Testing

```bash
# Run with persistence
cargo run -- --stdio --persist --storage-path /tmp/context-test

# Test storage
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"store","arguments":{"key":"test","value":"hello"}}}' | cargo run -- --stdio
```

### Integration with homelab-ci-stack

For lightweight text processing without persistence, the WASM version is available:

```bash
# Pull from GHCR
oras pull ghcr.io/tzervas/context-mcp:dev -o ./wasm/

# Test (stateless)
echo '{"method":"summarize","params":{"text":"..."}}' | wasmtime run ./wasm/context-mcp.wasm
```

## homelab-ci-stack Interop Data

The homelab-ci-stack test runner captures debug logs at:
```
logs/wasm-test-runs/<timestamp>/
├── interop/context-mcp-*.json  # Input/output pairs
├── oci/context-mcp-*.log       # OCI pull logs
└── artifacts/context-mcp/      # Downloaded WASM
```

Use these for developing compatible interfaces between native and WASM versions.

## Development Notes

### When to Use Which Version

| Scenario | Recommended |
|----------|-------------|
| Long-running workflow with state | Native (this repo) |
| One-shot text summarization | WASM (homelab-ci-stack) |
| Kubernetes deployment | Native (container) |
| Sandboxed agent tool | WASM (wasmtime) |
| RAG with large corpus | Native (parallel + disk) |

### Temporal Decay Algorithm

Context entries decay over time:
```
score = base_relevance * exp(-λ * age_hours)
```
Where λ is configurable decay rate. Older context naturally ranks lower in retrieval.

### Storage Tiers

1. **Hot** (LRU memory): Recent, frequently accessed
2. **Warm** (sled): Persistent, searchable
3. **Cold** (future): Archive to object storage

---

*This report was generated to provide cross-project context for AI-assisted development across the embeddenator ecosystem.*
