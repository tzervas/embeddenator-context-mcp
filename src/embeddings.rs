//! Mock and ternary embedding generation traits and implementations
//!
//! This module provides trait definitions for embedding generation with support for:
//! - Mock embeddings for testing
//! - Sparse balanced ternary embeddings (codebook-free and RVQ strategies)
//! - Quantized embeddings with optional GPU acceleration

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Trait for generating embeddings from text
#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    /// Generate an embedding vector from text
    async fn generate(&self, text: &str) -> Result<Vec<f32>>;

    /// Get the dimension of embeddings produced by this generator
    fn dimension(&self) -> usize;
}

/// Trait for quantized embeddings with reconstruction capability
#[async_trait]
pub trait QuantizedEmbeddingGenerator: Send + Sync {
    /// Generate a quantized embedding from text
    async fn generate_quantized(&self, text: &str) -> Result<QuantizedEmbedding>;

    /// Get the dimension of original embeddings
    fn dimension(&self) -> usize;

    /// Get the quantization strategy (e.g., "sparse", "rvq", "hybrid")
    fn strategy(&self) -> &str;

    /// Reconstruct the original embedding from quantized form
    async fn reconstruct(&self, quantized: &QuantizedEmbedding) -> Result<Vec<f32>>;
}

/// Quantized embedding representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantizedEmbedding {
    /// Sparse ternary embedding
    SparseTernary(crate::ternary::TernaryQuantizedEmbedding),
    /// Dense embedding (baseline)
    Dense(Vec<f32>),
}

impl QuantizedEmbedding {
    /// Get size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            Self::SparseTernary(sparse) => sparse.size_bytes(),
            Self::Dense(vec) => vec.len() * 4,
        }
    }
}

/// Mock embedding generator for testing and development
pub struct MockEmbeddingGenerator {
    dimension: usize,
}

impl MockEmbeddingGenerator {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl EmbeddingGenerator for MockEmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Generate deterministic embedding from text hash
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        let mut embedding = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let value = ((hash.wrapping_mul(i as u64 + 1)) as f32) / (u64::MAX as f32);
            embedding.push(value);
        }

        // Normalize the vector
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Ternary embedding generator with configurable quantization strategies
pub struct TernaryEmbeddingGeneratorWrapper {
    base_generator: Arc<dyn EmbeddingGenerator>,
    ternary_gen: Arc<crate::ternary::TernaryEmbeddingGenerator>,
}

impl TernaryEmbeddingGeneratorWrapper {
    /// Create with sparse ternary quantization
    pub fn with_sparse(
        base_generator: Arc<dyn EmbeddingGenerator>,
        config: crate::ternary::SparsityConfig,
    ) -> Self {
        let dimension = base_generator.dimension();
        let ternary_gen = Arc::new(crate::ternary::TernaryEmbeddingGenerator::with_sparse(
            dimension, config,
        ));

        Self {
            base_generator,
            ternary_gen,
        }
    }

    /// Create with RVQ quantization
    pub fn with_rvq(
        base_generator: Arc<dyn EmbeddingGenerator>,
        num_layers: usize,
        codebook_size: usize,
    ) -> Self {
        let dimension = base_generator.dimension();
        let ternary_gen = Arc::new(crate::ternary::TernaryEmbeddingGenerator::with_rvq(
            dimension,
            num_layers,
            codebook_size,
        ));

        Self {
            base_generator,
            ternary_gen,
        }
    }

    /// Create with hybrid quantization
    pub fn with_hybrid(
        base_generator: Arc<dyn EmbeddingGenerator>,
        sparse_config: crate::ternary::SparsityConfig,
        num_layers: usize,
        codebook_size: usize,
    ) -> Self {
        let dimension = base_generator.dimension();
        let ternary_gen = Arc::new(crate::ternary::TernaryEmbeddingGenerator::with_hybrid(
            dimension,
            sparse_config,
            num_layers,
            codebook_size,
        ));

        Self {
            base_generator,
            ternary_gen,
        }
    }
}

#[async_trait]
impl QuantizedEmbeddingGenerator for TernaryEmbeddingGeneratorWrapper {
    async fn generate_quantized(&self, text: &str) -> Result<QuantizedEmbedding> {
        let dense = self.base_generator.generate(text).await?;
        let quantized = self.ternary_gen.quantize(&dense)?;
        Ok(QuantizedEmbedding::SparseTernary(quantized))
    }

    fn dimension(&self) -> usize {
        self.base_generator.dimension()
    }

    fn strategy(&self) -> &str {
        &self.ternary_gen.strategy
    }

    async fn reconstruct(&self, quantized: &QuantizedEmbedding) -> Result<Vec<f32>> {
        match quantized {
            QuantizedEmbedding::SparseTernary(sparse) => self.ternary_gen.dequantize(sparse),
            QuantizedEmbedding::Dense(vec) => Ok(vec.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_deterministic() {
        let generator = MockEmbeddingGenerator::new(384);

        let emb1 = generator.generate("test text").await.unwrap();
        let emb2 = generator.generate("test text").await.unwrap();

        assert_eq!(emb1.len(), 384);
        assert_eq!(emb1, emb2); // Should be deterministic
    }

    #[tokio::test]
    async fn test_mock_embedding_normalized() {
        let generator = MockEmbeddingGenerator::new(384);
        let embedding = generator.generate("test").await.unwrap();

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001); // Should be unit length
    }

    #[tokio::test]
    async fn test_ternary_wrapper_sparse() {
        use crate::ternary::SparsityConfig;

        let base = Arc::new(MockEmbeddingGenerator::new(64));
        let config = SparsityConfig::default();
        let wrapper = TernaryEmbeddingGeneratorWrapper::with_sparse(base, config);

        let quantized = wrapper.generate_quantized("test").await.unwrap();
        let reconstructed = wrapper.reconstruct(&quantized).await.unwrap();

        assert_eq!(reconstructed.len(), 64);
    }

    #[tokio::test]
    async fn test_ternary_wrapper_rvq() {
        let base = Arc::new(MockEmbeddingGenerator::new(64));
        let wrapper = TernaryEmbeddingGeneratorWrapper::with_rvq(base, 2, 256);

        let quantized = wrapper.generate_quantized("test").await.unwrap();
        let reconstructed = wrapper.reconstruct(&quantized).await.unwrap();

        assert_eq!(reconstructed.len(), 64);
        assert_eq!(wrapper.strategy(), "rvq");
    }
}
