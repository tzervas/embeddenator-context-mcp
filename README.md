# embeddenator-context-mcp

MCP server for context management, temporal metadata, and lightweight retrieval/RAG plumbing.

This crate is intended to be a “memory service” for agents: store/retrieve context items with timestamps and metadata, and support simple query/retrieval patterns.

## Status

Alpha / under active development. APIs and tool names may change.

## What It Does Today (Code-Backed)

- Runs an MCP-compatible JSON-RPC server over HTTP/WebSocket.
- Stores context with IDs, domains, timestamps, and metadata.
- Supports tiered storage primitives (in-memory + on-disk via `sled`).

## What It Does Not Do Yet

- Production-grade embedding/vector search.
- Full RAG pipelines with reliable chunking/citation policies.
- Strong consistency guarantees across distributed replicas.

## Running

```bash
cargo run -p embeddenator-context-mcp -- --help
```

## License

MIT
