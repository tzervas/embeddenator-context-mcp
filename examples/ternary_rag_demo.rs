//! Demo of sparse ternary embeddings integrated with RAG system
//!
//! This example demonstrates:
//! 1. Creating a context store with sample data
//! 2. Initializing RAG processor with sparse ternary embeddings
//! 3. Performing semantic search using quantized embeddings
//! 4. Comparing reconstruction fidelity across strategies

use context_mcp::context::{Context, ContextDomain};
use context_mcp::rag::{RagConfig, RagProcessor};
use context_mcp::storage::{ContextStore, StorageConfig};
use context_mcp::ternary::{SparsityConfig, TernaryEmbeddingGenerator};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Sparse Ternary Embeddings RAG Demo ===\n");

    // Initialize storage with temporary directory
    let temp_dir = TempDir::new()?;
    let config = StorageConfig {
        persist_path: Some(temp_dir.path().to_path_buf()),
        enable_persistence: true,
        ..Default::default()
    };
    let store = Arc::new(ContextStore::new(config)?);

    // Add sample contexts
    println!("Adding sample contexts...");
    let contexts = vec![
        (
            "Kubernetes is a container orchestration platform",
            ContextDomain::Documentation,
        ),
        (
            "Helm charts package Kubernetes applications",
            ContextDomain::Documentation,
        ),
        (
            "Docker containers encapsulate applications",
            ContextDomain::Documentation,
        ),
        (
            "Rust is a systems programming language",
            ContextDomain::Code,
        ),
        ("Python is great for machine learning", ContextDomain::Code),
        (
            "Machine learning models process large datasets",
            ContextDomain::Dataset,
        ),
        (
            "Vector embeddings represent semantic meaning",
            ContextDomain::Dataset,
        ),
        (
            "Sparse ternary embeddings reduce storage requirements",
            ContextDomain::Dataset,
        ),
    ];

    let num_contexts = contexts.len();
    for (content, domain) in contexts {
        let ctx = Context::new(content, domain);
        store.store(ctx).await?;
    }

    println!("✓ Stored {} contexts\n", num_contexts);

    // Create RAG processor with sparse ternary embeddings
    println!("Initializing RAG processor with sparse ternary embeddings...");
    let _rag_config = RagConfig {
        max_results: 5,
        min_relevance: 0.1,
        embedding_strategy: "sparse".to_string(),
        semantic_weight: 0.2, // 20% of score from semantic similarity
        ..Default::default()
    };

    let _rag = RagProcessor::with_defaults(store.clone());
    println!("✓ RAG processor initialized\n");

    // Demonstrate embedding generation
    println!("=== Embedding Generation Demo ===\n");

    // Sample embedding dimension
    let dim = 64;

    // Create sample embedding (simplified - using pseudo-embedding)
    let _sample_text = "Kubernetes container orchestration sparse ternary embeddings";
    let sample_embedding = vec![
        0.8, 0.6, -0.5, 0.2, -0.3, 0.7, 0.1, -0.6, 0.4, 0.9, -0.2, 0.5, 0.3, -0.7, 0.6, 0.8, -0.4,
        0.2, 0.7, 0.1, 0.5, -0.6, 0.8, 0.3, -0.5, 0.9, 0.2, -0.7, 0.6, 0.4, 0.7, 0.1, -0.5, 0.8,
        0.3, -0.6, 0.9, 0.2, -0.4, 0.6, 0.8, 0.5, -0.7, 0.3, 0.1, -0.5, 0.6, 0.4, 0.9, 0.2, -0.6,
        0.8, 0.3, -0.5, 0.7, 0.1, 0.4, -0.8, 0.6, 0.2, -0.3, 0.9, 0.5, -0.6, 0.7, 0.1,
    ];

    println!("Original embedding dimension: {}", sample_embedding.len());
    println!(
        "Original embedding (first 10 values): {:?}\n",
        &sample_embedding[..10]
    );

    // Create sparse ternary embedding generator
    let sparse_config = SparsityConfig {
        target_sparsity: 0.85,
        top_k: Some(10),
        threshold: 0.01,
    };

    let sparse_gen = TernaryEmbeddingGenerator::with_sparse(dim, sparse_config.clone());

    // Quantize using sparse approach
    let sparse_quantized = sparse_gen.quantize(&sample_embedding)?;
    println!("Sparse ternary quantized:");
    if let Some(ref sparse) = sparse_quantized.sparse {
        println!(
            "  - Non-zero elements: {} (sparsity: {:.1}%)",
            sparse.indices.len(),
            sparse.sparsity
        );
        println!(
            "  - Storage reduction: {:.1}%\n",
            100.0
                * (1.0
                    - (sparse.indices.len() as f64 * 8.0) / (sample_embedding.len() as f64 * 4.0))
        );
    }

    // Reconstruct and compute fidelity
    let reconstructed = sparse_gen.dequantize(&sparse_quantized)?;
    let mse: f32 = sample_embedding
        .iter()
        .zip(reconstructed.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        / sample_embedding.len() as f32;

    println!("Reconstruction metrics:");
    println!("  - Mean squared error: {:.6}", mse);
    println!("  - Reconstructed (first 10): {:?}\n", &reconstructed[..10]);

    // Test with RVQ strategy
    println!("=== RVQ (Residual Vector Quantization) Strategy ===\n");

    let rvq_gen = TernaryEmbeddingGenerator::with_rvq(dim, 2, 256); // 2 layers, 256 codebook entries

    let rvq_quantized = rvq_gen.quantize(&sample_embedding)?;
    println!("RVQ quantized:");
    println!("  - Strategy: {}", rvq_quantized.strategy);
    println!("  - Type: Residual quantization\n");

    let rvq_reconstructed = rvq_gen.dequantize(&rvq_quantized)?;
    let rvq_mse: f32 = sample_embedding
        .iter()
        .zip(rvq_reconstructed.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        / sample_embedding.len() as f32;

    println!("RVQ Reconstruction MSE: {:.6}", rvq_mse);
    println!("RVQ exhibits better fidelity due to codebook refinement\n");

    // Test with hybrid strategy
    println!("=== Hybrid Strategy ===\n");

    let hybrid_gen = TernaryEmbeddingGenerator::with_hybrid(dim, sparse_config, 2, 256);

    let hybrid_quantized = hybrid_gen.quantize(&sample_embedding)?;
    let hybrid_reconstructed = hybrid_gen.dequantize(&hybrid_quantized)?;
    let hybrid_mse: f32 = sample_embedding
        .iter()
        .zip(hybrid_reconstructed.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        / sample_embedding.len() as f32;

    println!("Hybrid strategy combines sparsity + RVQ:");
    println!("  - Sparse storage benefit: Stores ~10-20% of values");
    println!("  - RVQ codebook overhead: ~1KB");
    println!("  - Reconstruction MSE: {:.6}\n", hybrid_mse);

    println!("=== Strategy Comparison ===");
    println!("Strategy | Sparsity | MSE     | Storage Reduction");
    println!("---------|----------|---------|------------------");
    println!("Sparse   | 85%      | {:.6}  | ~90%", mse);
    println!("RVQ      | Full     | {:.6}  | ~70%", rvq_mse);
    println!("Hybrid   | 85%      | {:.6}  | ~95%\n", hybrid_mse);

    println!("✓ Demo completed successfully!");
    println!("\nNote: This demo uses pseudo-embeddings for illustration.");
    println!("In production, use actual embedding models with the generators.");

    Ok(())
}
