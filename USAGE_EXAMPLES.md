# Context-MCP Usage Examples

This document shows how to use the context-mcp tools that are now available in VS Code.

## Available Tools

The following tools are accessible through VS Code's MCP integration:

1. **store_context** - Store information for later retrieval
2. **get_context** - Retrieve stored context by ID
3. **delete_context** - Remove contexts
4. **query_contexts** - Search with filters
5. **retrieve_contexts** - RAG-based retrieval
6. **update_screening** - Update security status
7. **get_temporal_stats** - Temporal analytics
8. **get_storage_stats** - Storage metrics
9. **cleanup_expired** - Remove expired items

## Example Usage Scenarios

### Scenario 1: Store Code Snippets

You can store code examples or patterns for later reference:

```
Store this context:
"Async/await pattern in Python for concurrent I/O operations"
Domain: Code
Tags: python, async, patterns
Importance: 0.8
```

### Scenario 2: Query Stored Contexts

Search for stored information by domain or tags:

```
Query contexts where:
- Domain: Code
- Tags: python
- Minimum importance: 0.7
```

### Scenario 3: RAG Retrieval

Use natural language to find relevant contexts:

```
Retrieve contexts about: "python async patterns"
Max results: 5
Domain: Code
```

### Scenario 4: Temporal Tracking

Get statistics about your stored contexts:

```
Get temporal stats to see:
- Total contexts stored
- Contexts by domain
- Age distribution
- Access patterns
```

## Testing the Setup

To verify everything is working, you can:

1. **Check if tools are available**: The context-mcp tools should appear in your VS Code MCP tools list
2. **Store a test context**: Try storing a simple text context
3. **Query it back**: Use query_contexts to find what you stored
4. **Check storage stats**: View metrics about your context store

## Performance Characteristics

Based on benchmarking:

- **Storage**: ~7,400 contexts/second
- **Retrieval**: Sub-millisecond latency (0.14-0.23ms)
- **Query**: Sub-millisecond with filters (0.16-0.18ms)
- **RAG Retrieval**: ~0.23ms average

## Configuration

The server is configured in your VS Code MCP settings:

```json
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "/home/kang/.local/bin/context-mcp",
      "args": ["--stdio"]
    }
  }
}
```

### Advanced Options

You can modify the configuration to enable additional features:

```json
{
  "args": [
    "--stdio",
    "--persist",                          // Enable disk persistence
    "--storage-path", "/path/to/data",    // Persistent storage location
    "--cache-size", "10000",              // Increase cache size
    "--threads", "8"                      // Set thread pool size
  ]
}
```

## Best Practices

1. **Use meaningful tags**: Tag contexts with searchable keywords
2. **Set importance scores**: Use 0.0-1.0 to prioritize contexts
3. **Specify domains**: Categorize by Code, Documentation, Research, etc.
4. **Add source attribution**: Track where contexts came from
5. **Use TTL for temporary data**: Set expiration times for transient contexts

## Troubleshooting

If the tools aren't appearing:

1. Reload VS Code window (Ctrl+Shift+P → "Developer: Reload Window")
2. Check MCP server status in VS Code
3. Verify binary exists: `ls -lh ~/.local/bin/context-mcp`
4. Test manually: `~/.local/bin/context-mcp --help`

## Full Test Results

See `ASSESSMENT_REPORT.md` for comprehensive benchmark results and validation.
