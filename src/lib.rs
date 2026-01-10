//! # Context MCP Server
//!
//! A Model Context Protocol (MCP) server for context management, RAG processing,
//! and temporal reasoning. Inspired by memory-gate patterns for dynamic learning layers.
//!
//! ## Features
//!
//! - **Multi-tier Storage**: In-memory (LRU), cache, and disk persistence
//! - **Temporal Reasoning**: Timestamps and age tracking for context relevance
//! - **RAG Processing**: CPU-optimized retrieval-augmented generation support
//! - **Safe Input Handling**: Integrates with security-mcp for screened inputs
//! - **MCP Protocol**: Full compatibility with VS Code, Copilot, and other MCP clients
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   MCP Client    │    │  Context Gateway │    │ Storage Layer   │
//! │                 │    │                  │    │                 │
//! │ • VS Code       │◄──►│ • Store/Retrieve │◄──►│ • In-Memory LRU │
//! │ • Copilot       │    │ • Temporal Query │    │ • Sled Disk DB  │
//! │ • CLI Tools     │    │ • RAG Processing │    │ • Vector Index  │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//! ```

pub mod context;
pub mod embeddings;
pub mod error;
pub mod protocol;
pub mod rag;
pub mod server;
pub mod storage;
pub mod temporal;
pub mod tools;

pub use context::{Context, ContextId, ContextMetadata};
pub use error::{ContextError, Result};
pub use server::{McpServer, ServerConfig};
pub use storage::{ContextStore, StorageConfig};
pub use temporal::TemporalQuery;
