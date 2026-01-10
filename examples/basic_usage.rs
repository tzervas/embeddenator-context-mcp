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
    println!("Stored context with ID: {}", id);

    // Retrieve it
    let retrieved = store.get(&id).await?;
    println!("Retrieved: {}", retrieved.content);

    // Query contexts
    let results = store.query_text("important", 10, None).await?;
    println!("Found {} matching contexts", results.len());

    Ok(())
}
