# RMCP (Rust MCP SDK) Security Audit Plan

**Date:** January 9, 2026  
**Target:** https://github.com/modelcontextprotocol/rust-sdk  
**Version:** v0.12.0 (latest as of audit date)  
**License:** MIT

---

## 1. Clone Repository

```bash
cd /home/kang/Documents/projects/2026/mcp
git clone https://github.com/modelcontextprotocol/rust-sdk.git rmcp-audit
cd rmcp-audit
```

---

## 2. Security Audit Areas

### 2.1 Unsafe Code Audit
```bash
# Find all unsafe blocks
grep -rn "unsafe" crates/rmcp/src/
grep -rn "unsafe" crates/rmcp-macros/src/

# Check for unsafe in dependencies
cargo geiger  # Install: cargo install cargo-geiger
```

**Focus areas:**
- Any `unsafe` blocks in transport layer
- FFI boundaries (if any)
- Raw pointer usage
- Transmutation

### 2.2 Deserialization Attack Surface
**Files to audit:**
- `crates/rmcp/src/model.rs` - All JSON-RPC message types
- `crates/rmcp/src/model/serde_impl.rs` - Custom deserializers
- `crates/rmcp/src/transport/async_rw.rs` - Message codec

**Red flags to check:**
- [ ] Unbounded allocations on malicious input
- [ ] Stack overflow via recursive structures
- [ ] Integer overflow in length fields
- [ ] Denial of service via large payloads
- [ ] Type confusion in tagged unions

**Specific concerns found in code search:**
- `serde_json::from_value` used extensively - check for panic paths
- `serde_json::from_slice` in codec - verify size limits
- `CallToolResult` requires validation: "must have either content or structured_content"

### 2.3 Network/Transport Security
**Files to audit:**
- `crates/rmcp/src/transport/streamable_http_client.rs`
- `crates/rmcp/src/transport/streamable_http_server/`
- `crates/rmcp/src/transport/auth.rs` (~1500 lines, OAuth impl)
- `crates/rmcp/src/transport/child_process.rs`
- `crates/rmcp/src/transport/io.rs`

**Checklist:**
- [ ] TLS certificate validation (look for `danger_accept_invalid_certs`)
- [ ] HTTP redirect handling (found: `Policy::none()` in auth.rs - GOOD)
- [ ] Connection timeout handling (found: 30s default - CHECK)
- [ ] SSE reconnection logic - rate limiting?
- [ ] Session ID handling and validation
- [ ] CSRF token entropy and validation

### 2.4 OAuth/Authentication Audit
**File:** `crates/rmcp/src/transport/auth.rs`

**Specific checks:**
- [ ] PKCE implementation correctness
- [ ] State parameter entropy (uses `CsrfToken` from oauth2 crate)
- [ ] Token storage security (in-memory by default)
- [ ] Redirect URI validation
- [ ] Client registration security
- [ ] SEP-991 URL-based client ID validation (`is_https_url` function)
- [ ] WWW-Authenticate header parsing for injection

### 2.5 Credential/Secret Handling
**Search for:**
```bash
grep -rn "password\|secret\|token\|key\|credential" crates/rmcp/src/
```

**Known patterns found:**
- `InMemoryCredentialStore` - credentials stored in `Arc<RwLock<Option<StoredCredentials>>>`
- `CredentialStore` trait for custom storage
- OAuth tokens serialized via serde

### 2.6 Dependency Audit
```bash
# Security vulnerabilities
cargo audit  # Install: cargo install cargo-audit

# Dependency tree analysis
cargo tree --duplicates
cargo deny check  # If deny.toml exists

# Check for concerning dependencies
cargo tree | grep -E "openssl|native-tls|ring|rustls"
```

**Known dependencies (from code):**
- `tokio` - async runtime
- `serde` / `serde_json` - serialization
- `reqwest` - HTTP client
- `oauth2` - OAuth implementation
- `url` - URL parsing
- `thiserror` - error handling

---

## 3. Protocol Compliance Verification

### 3.1 MCP Specification Alignment
**Reference:** https://modelcontextprotocol.io/specification/2025-11-25

**Protocol versions supported:**
- `2024-11-05`
- `2025-03-26`  
- `2025-06-18`

**Verify implementation of:**
- [ ] JSON-RPC 2.0 message format
- [ ] Initialize handshake sequence
- [ ] Capability negotiation
- [ ] All required request/response types
- [ ] Notification handling
- [ ] Error response format

### 3.2 Required Trait Verification
**Core traits to verify:**
```rust
// From crates/rmcp/src/transport.rs
pub trait Transport<R>: Send {
    type Error;
    fn send(&mut self, item) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static;
    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<R>>> + Send;
    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

// From crates/rmcp/src/service/server.rs  
pub trait Service<R: ServiceRole>: Send + Sync + 'static { ... }

// From handler modules
pub trait ServerHandler { ... }
pub trait ClientHandler { ... }
```

### 3.3 Transport Implementations
| Transport | Client | Server |
|-----------|--------|--------|
| stdio | `TokioChildProcess` | `io::stdio` |
| Streamable HTTP | `StreamableHttpClientTransport` | `StreamableHttpService` |

**Additional transports in examples (not core):**
- WebSocket
- Unix socket
- Named pipe (Windows)
- TCP
- HTTP upgrade

---

## 4. Red Flags Checklist

### 4.1 Code Quality
- [ ] No obfuscated code
- [ ] Clear error messages (not leaking internal state)
- [ ] Proper use of `#[deny(unsafe_code)]` where possible
- [ ] Test coverage for security-sensitive code

### 4.2 Suspicious Patterns
```bash
# External network calls outside documented transports
grep -rn "http://" crates/rmcp/src/
grep -rn "reqwest::get\|reqwest::post" crates/rmcp/src/

# File system access
grep -rn "std::fs\|tokio::fs\|File::" crates/rmcp/src/

# Process execution
grep -rn "Command::\|std::process" crates/rmcp/src/

# Environment variable access
grep -rn "std::env\|env!" crates/rmcp/src/
```

### 4.3 Macro Safety
**File:** `crates/rmcp-macros/src/`

- [ ] `#[tool_router]` macro - verify no code injection
- [ ] `#[tool]` attribute - parameter handling
- [ ] Generated code inspection

---

## 5. Integration Testing Plan

### 5.1 Test Against Reference Servers
```bash
# Run against official MCP servers
cargo run --example clients_git_stdio
cargo run --example clients_streamable_http
```

### 5.2 Fuzz Testing (Optional)
```bash
# For deserialization paths
cargo +nightly fuzz run <target>  # If fuzz targets exist
```

### 5.3 Protocol Conformance
```bash
# Run SDK test suite
cd crates/rmcp
cargo test --all-features
```

---

## 6. Key Files for Deep Review

**Priority 1 (Security Critical):**
1. `crates/rmcp/src/transport/auth.rs` - OAuth implementation
2. `crates/rmcp/src/model.rs` - Message type definitions
3. `crates/rmcp/src/transport/async_rw.rs` - Codec/parsing
4. `crates/rmcp/src/service/server.rs` - Server initialization

**Priority 2 (Integration Critical):**
5. `crates/rmcp/src/lib.rs` - Public API surface
6. `crates/rmcp/src/handler/server.rs` - Handler traits
7. `crates/rmcp/src/transport.rs` - Transport traits

**Priority 3 (Macro Safety):**
8. `crates/rmcp-macros/src/lib.rs` - Proc macros

---

## 7. Audit Execution Commands

```bash
# Full audit workflow
cd /home/kang/Documents/projects/2026/mcp/rmcp-audit

# 1. Dependency security
cargo audit
cargo deny check 2>/dev/null || echo "No deny.toml"

# 2. Unsafe code scan
cargo geiger --all-features 2>/dev/null || grep -rn "unsafe" crates/

# 3. Build and test
cargo build --all-features
cargo test --all-features

# 4. Code stats
tokei crates/rmcp/src/
cloc crates/rmcp/src/

# 5. Security patterns
grep -rn "panic!\|unwrap()\|expect(" crates/rmcp/src/ | wc -l
```

---

## 8. Expected Findings Summary

**Positive indicators from initial review:**
- ✅ Official repository under `modelcontextprotocol` org
- ✅ MIT license (permissive, auditable)
- ✅ 117 contributors, 2.8k stars, active development
- ✅ Uses well-known crates (tokio, serde, oauth2, reqwest)
- ✅ Comprehensive test suite visible
- ✅ OAuth uses PKCE (from oauth2 crate)
- ✅ HTTP redirects disabled in auth flow

**Areas requiring deeper review:**
- ⚠️ Large OAuth implementation (~1500 lines)
- ⚠️ Custom serde implementations for protocol messages
- ⚠️ SSE streaming reconnection logic
- ⚠️ Proc macros need code generation review

---

## 9. Integration Decision Matrix

| Finding | Impact | Action |
|---------|--------|--------|
| No unsafe code | Low risk | Proceed |
| Minimal unsafe, justified | Accept | Document |
| Suspicious network calls | High risk | Block |
| Obfuscated code | Critical | Reject |
| Deserialization DoS possible | Medium | Patch/Report |
| OAuth implementation bugs | High | Fork + fix or wait |

---

## Next Steps After Audit

1. **If PASS:** Create `rmcp` integration branch
2. **If ISSUES:** Open issues on upstream, evaluate workarounds
3. **Document:** Add security notes to `context-mcp` README
