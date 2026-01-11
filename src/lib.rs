//! # Context MCP Server
//!
//! A Model Context Protocol (MCP) server for context storage, text-based retrieval,
//! and temporal tracking.
//!
//! ## Features
//!
//! - **Multi-tier Storage**: In-memory LRU cache with optional sled disk persistence
//! - **Temporal Tracking**: Timestamps, age tracking, and time-based filtering
//! - **Text-Based Retrieval**: CPU-optimized parallel text matching and scoring
//! - **Screening Status**: Fields for tracking security screening state (no active integration)
//! - **MCP Protocol**: JSON-RPC server with HTTP/WebSocket and stdio transports
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   MCP Client    │    │  JSON-RPC Server │    │ Storage Layer   │
//! │                 │    │                  │    │                 │
//! │ • HTTP/WS       │◄──►│ • Store/Retrieve │◄──►│ • In-Memory LRU │
//! │ • stdio         │    │ • Query/Filter   │    │ • Sled (opt)    │
//! │ • curl/tools    │    │ • Text Matching  │    │ • Indexes       │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//! ```

pub mod context;
pub mod embeddings;
pub mod error;
#[cfg(feature = "gpu-acceleration")]
pub mod gpu;
pub mod protocol;
pub mod rag;
#[cfg(feature = "server")]
pub mod server;
pub mod storage;
pub mod temporal;
pub mod ternary;
pub mod tools;

pub use context::{Context, ContextId, ContextMetadata};
pub use error::{ContextError, Result};
#[cfg(feature = "server")]
pub use server::{McpServer, ServerConfig};
pub use storage::{ContextStore, StorageConfig};
pub use temporal::TemporalQuery;
