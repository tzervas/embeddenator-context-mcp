//! Mock embedding generation traits and implementations
//!
//! This module provides trait definitions for embedding generation that will
//! be replaced with real implementations (candle, embeddenator-vsa) in future PRs.

use crate::error::Result;
use async_trait::async_trait;

/// Trait for generating embeddings from text
#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    /// Generate an embedding vector from text
    async fn generate(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Get the dimension of embeddings produced by this generator
    fn dimension(&self) -> usize;
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
}
