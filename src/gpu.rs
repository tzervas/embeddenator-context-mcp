//! Optional GPU acceleration for ternary embeddings
//!
//! This module provides GPU-accelerated computation for large-scale
//! similarity searches and quantization operations using wgpu.
//!
//! When the `gpu-acceleration` feature is enabled, operations can optionally
//! use GPU compute shaders. CPU fallback is always available.

#[cfg(feature = "gpu-acceleration")]
use wgpu::*;

/// GPU computation backend trait
pub trait GpuBackend: Send + Sync {
    /// Check if GPU is available
    fn is_available(&self) -> bool;

    /// Compute cosine similarity between embeddings on GPU
    fn cosine_similarity_batch(
        &self,
        query: &[f32],
        candidates: &[Vec<f32>],
    ) -> Result<Vec<f32>, String>;
}

/// CPU fallback implementation
pub struct CpuBackend;

impl GpuBackend for CpuBackend {
    fn is_available(&self) -> bool {
        false
    }

    fn cosine_similarity_batch(
        &self,
        query: &[f32],
        candidates: &[Vec<f32>],
    ) -> Result<Vec<f32>, String> {
        // CPU implementation
        let query_norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
        if query_norm == 0.0 {
            return Ok(vec![0.0; candidates.len()]);
        }

        Ok(candidates
            .iter()
            .map(|cand| {
                let dot: f32 = query.iter().zip(cand.iter()).map(|(a, b)| a * b).sum();
                let cand_norm: f32 = cand.iter().map(|x| x * x).sum::<f32>().sqrt();
                if cand_norm == 0.0 {
                    0.0
                } else {
                    (dot / (query_norm * cand_norm)).clamp(-1.0, 1.0)
                }
            })
            .collect())
    }
}

#[cfg(feature = "gpu-acceleration")]
pub struct WgpuBackend {
    device: Device,
    queue: Queue,
}

#[cfg(feature = "gpu-acceleration")]
impl WgpuBackend {
    /// Initialize GPU backend (async)
    pub async fn new() -> Result<Self, String> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
            flags: Default::default(),
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| {
                eprintln!("GPU initialization failed: No suitable GPU adapter found. Falling back to CPU.");
                "No GPU adapter found".to_string()
            })?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await
            .map_err(|e| {
                eprintln!(
                    "GPU initialization failed: Failed to create device: {}. Falling back to CPU.",
                    e
                );
                format!("Failed to create device: {}", e)
            })?;

        Ok(Self { device, queue })
    }

    /// Get device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}

#[cfg(feature = "gpu-acceleration")]
impl GpuBackend for WgpuBackend {
    fn is_available(&self) -> bool {
        true
    }

    fn cosine_similarity_batch(
        &self,
        _query: &[f32],
        _candidates: &[Vec<f32>],
    ) -> Result<Vec<f32>, String> {
        // Placeholder: in production, implement compute shader for similarity
        // For now, fall back to CPU
        Err("GPU compute shader not yet implemented".to_string())
    }
}

/// GPU computation wrapper with automatic fallback
pub struct GpuCompute {
    #[cfg(feature = "gpu-acceleration")]
    gpu: Option<WgpuBackend>,
    cpu: CpuBackend,
}

impl GpuCompute {
    /// Create GPU compute with auto-detection
    #[cfg(feature = "gpu-acceleration")]
    pub async fn new(_prefer_gpu: bool) -> Self {
        let gpu = WgpuBackend::new().await.ok();

        Self {
            gpu,
            cpu: CpuBackend,
        }
    }

    /// Create CPU-only compute
    #[cfg(not(feature = "gpu-acceleration"))]
    pub async fn new(_prefer_gpu: bool) -> Self {
        Self { cpu: CpuBackend }
    }

    /// Compute cosine similarity with GPU if available, CPU fallback
    pub fn cosine_similarity_batch(
        &self,
        query: &[f32],
        candidates: &[Vec<f32>],
    ) -> Result<Vec<f32>, String> {
        #[cfg(feature = "gpu-acceleration")]
        {
            if let Some(ref gpu) = self.gpu {
                match gpu.cosine_similarity_batch(query, candidates) {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        eprintln!(
                            "Warning: GPU acceleration failed ({}), falling back to CPU.",
                            e
                        );
                    }
                }
            }
        }

        self.cpu.cosine_similarity_batch(query, candidates)
    }

    /// Check if GPU is currently available
    pub fn is_gpu_available(&self) -> bool {
        #[cfg(feature = "gpu-acceleration")]
        {
            self.gpu.is_some()
        }
        #[cfg(not(feature = "gpu-acceleration"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_backend_similarity() {
        let backend = CpuBackend;
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.707, 0.707, 0.0],
        ];

        let result = backend
            .cosine_similarity_batch(&query, &candidates)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 1.0).abs() < 0.01);
        assert!((result[1] - 0.0).abs() < 0.01);
        assert!((result[2] - 0.707).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_gpu_compute_fallback() {
        let compute = GpuCompute::new(false).await;
        let query = vec![1.0, 0.0];
        let candidates = vec![vec![1.0, 0.0], vec![0.0, 1.0]];

        let result = compute.cosine_similarity_batch(&query, &candidates);
        assert!(result.is_ok());
    }
}
