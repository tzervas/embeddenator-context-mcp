use context_mcp::{
    context::ContextDomain,
    embeddings::{
        EmbeddingGenerator, MockEmbeddingGenerator, QuantizedEmbeddingGenerator,
        TernaryEmbeddingGeneratorWrapper,
    },
    rag::{RagProcessor, RetrievalQuery},
    ternary::{SparsityConfig, TernarySimilarity},
    Context, ContextStore, StorageConfig,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Benchmark ternary embedding quantization and reconstruction
fn ternary_quantization_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("ternary_quantization");

    for dimension in [64, 384, 768].iter() {
        // Benchmark sparse quantization
        group.bench_with_input(
            BenchmarkId::new("sparse_quantize", dimension),
            dimension,
            |b, &dim| {
                let base_gen = Arc::new(MockEmbeddingGenerator::new(dim));
                let gen = TernaryEmbeddingGeneratorWrapper::with_sparse(
                    base_gen,
                    SparsityConfig::default(),
                );

                b.to_async(&rt).iter(|| async {
                    let _quantized = gen.generate_quantized("test embedding").await.unwrap();
                });
            },
        );

        // Benchmark RVQ quantization
        group.bench_with_input(
            BenchmarkId::new("rvq_quantize", dimension),
            dimension,
            |b, &dim| {
                let base_gen = Arc::new(MockEmbeddingGenerator::new(dim));
                let gen = TernaryEmbeddingGeneratorWrapper::with_rvq(base_gen, 2, 256);

                b.to_async(&rt).iter(|| async {
                    let _quantized = gen.generate_quantized("test embedding").await.unwrap();
                });
            },
        );

        // Benchmark reconstruction
        group.bench_with_input(
            BenchmarkId::new("sparse_reconstruct", dimension),
            dimension,
            |b, &dim| {
                let base_gen = Arc::new(MockEmbeddingGenerator::new(dim));
                let gen = TernaryEmbeddingGeneratorWrapper::with_sparse(
                    base_gen,
                    SparsityConfig::default(),
                );

                b.to_async(&rt).iter(|| async {
                    let quantized = gen.generate_quantized("test").await.unwrap();
                    let _reconstructed = gen.reconstruct(&quantized).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sparse ternary similarity computation
fn sparse_similarity_benchmarks(c: &mut Criterion) {
    use context_mcp::ternary::SparseTernaryEmbedding;

    let mut group = c.benchmark_group("sparse_similarity");

    for sparsity in ["low", "medium", "high"].iter() {
        let (indices_a, values_a) = match *sparsity {
            "low" => {
                let idx: Vec<u32> = (0..200).step_by(1).map(|i| i as u32).collect();
                let vals: Vec<i8> = (0..200)
                    .step_by(1)
                    .map(|i| if i % 2 == 0 { 1 } else { -1 })
                    .collect();
                (idx, vals)
            }
            "medium" => {
                let idx: Vec<u32> = (0..200).step_by(2).map(|i| i as u32).collect();
                let vals: Vec<i8> = (0..100).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();
                (idx, vals)
            }
            _ => {
                let idx: Vec<u32> = (0..384).step_by(10).map(|i| i as u32).collect();
                let vals: Vec<i8> = (0..39).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();
                (idx, vals)
            }
        };

        let embedding_a =
            SparseTernaryEmbedding::new(384, indices_a.clone(), values_a.clone()).unwrap();
        let embedding_b =
            SparseTernaryEmbedding::new(384, indices_a.clone(), values_a.clone()).unwrap();

        group.bench_with_input(BenchmarkId::new("cosine", sparsity), sparsity, |b, _| {
            b.iter(|| {
                let _sim = TernarySimilarity::cosine_sparse(
                    black_box(&embedding_a),
                    black_box(&embedding_b),
                );
            });
        });

        group.bench_with_input(BenchmarkId::new("hamming", sparsity), sparsity, |b, _| {
            b.iter(|| {
                let _sim = TernarySimilarity::hamming_sparse(
                    black_box(&embedding_a),
                    black_box(&embedding_b),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark RAG retrieval with different dataset sizes
fn rag_dataset_size_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("rag_dataset_sizes");

    for dataset_size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(dataset_size),
            dataset_size,
            |b, &dataset_size| {
                b.to_async(&rt).iter(|| async move {
                    let config = StorageConfig {
                        memory_cache_size: 10000,
                        persist_path: None,
                        auto_cleanup: false,
                        cleanup_interval_secs: 3600,
                        enable_persistence: false,
                    };
                    let store = Arc::new(ContextStore::new(config).unwrap());
                    let rag = RagProcessor::with_defaults(store.clone());

                    // Pre-populate with contexts
                    for i in 0..dataset_size {
                        let ctx = Context::new(
                            format!(
                                "Important information about topic {} with details and content",
                                i
                            ),
                            ContextDomain::Code,
                        );
                        store.store(ctx).await.unwrap();
                    }

                    // Perform retrieval
                    let query = RetrievalQuery::from_text("information");
                    let _result = rag.retrieve(black_box(&query)).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory efficiency of quantized embeddings
fn embedding_memory_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("embedding_memory");

    for num_embeddings in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("sparse_storage", num_embeddings),
            num_embeddings,
            |b, &num| {
                b.to_async(&rt).iter(|| async {
                    let base_gen = Arc::new(MockEmbeddingGenerator::new(384));
                    let gen = TernaryEmbeddingGeneratorWrapper::with_sparse(
                        base_gen,
                        SparsityConfig::default(),
                    );

                    let mut total_size = 0;
                    for i in 0..num {
                        let quantized = gen
                            .generate_quantized(&format!("text {}", i))
                            .await
                            .unwrap();
                        total_size += quantized.size_bytes();
                    }
                    let _avg_size = total_size / num;
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rvq_storage", num_embeddings),
            num_embeddings,
            |b, &num| {
                b.to_async(&rt).iter(|| async {
                    let base_gen = Arc::new(MockEmbeddingGenerator::new(384));
                    let gen = TernaryEmbeddingGeneratorWrapper::with_rvq(base_gen, 2, 256);

                    let mut total_size = 0;
                    for i in 0..num {
                        let quantized = gen
                            .generate_quantized(&format!("text {}", i))
                            .await
                            .unwrap();
                        total_size += quantized.size_bytes();
                    }
                    let _avg_size = total_size / num;
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("dense_storage", num_embeddings),
            num_embeddings,
            |b, &num| {
                b.to_async(&rt).iter(|| async {
                    let gen = Arc::new(MockEmbeddingGenerator::new(384));

                    let mut total_size = 0;
                    for i in 0..num {
                        let embedding = gen.generate(&format!("text {}", i)).await.unwrap();
                        total_size += embedding.len() * 4; // 4 bytes per f32
                    }
                    let _avg_size = total_size / num;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark reconstruction fidelity
fn reconstruction_fidelity_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("reconstruction_fidelity");

    for strategy in ["sparse", "rvq", "hybrid"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(strategy),
            strategy,
            |b, &strat| {
                b.to_async(&rt).iter(|| async {
                    let base_gen = Arc::new(MockEmbeddingGenerator::new(384));

                    let gen: Arc<dyn QuantizedEmbeddingGenerator> = match strat {
                        "sparse" => Arc::new(TernaryEmbeddingGeneratorWrapper::with_sparse(
                            base_gen,
                            SparsityConfig::default(),
                        )),
                        "rvq" => {
                            Arc::new(TernaryEmbeddingGeneratorWrapper::with_rvq(base_gen, 2, 256))
                        }
                        _ => Arc::new(TernaryEmbeddingGeneratorWrapper::with_hybrid(
                            base_gen,
                            SparsityConfig::default(),
                            2,
                            256,
                        )),
                    };

                    let original = gen.dimension();
                    let mut mse_sum = 0.0;

                    for i in 0..10 {
                        let original_emb = MockEmbeddingGenerator::new(original)
                            .generate(&format!("test {}", i))
                            .await
                            .unwrap();

                        let quantized = gen
                            .generate_quantized(&format!("test {}", i))
                            .await
                            .unwrap();
                        let reconstructed = gen.reconstruct(&quantized).await.unwrap();

                        let mse: f32 = original_emb
                            .iter()
                            .zip(reconstructed.iter())
                            .map(|(a, b)| (a - b).powi(2))
                            .sum();
                        mse_sum += mse;
                    }

                    let _avg_mse = mse_sum / 10.0;
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    ternary_quantization_benchmarks,
    sparse_similarity_benchmarks,
    rag_dataset_size_benchmarks,
    embedding_memory_benchmarks,
    reconstruction_fidelity_benchmarks
);
criterion_main!(benches);
