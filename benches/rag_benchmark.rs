use context_mcp::{
    context::ContextDomain,
    rag::{RagProcessor, RetrievalQuery},
    Context, ContextStore, StorageConfig,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn rag_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Benchmark: RAG retrieval with different dataset sizes
    let mut group = c.benchmark_group("rag_retrieval");
    for dataset_size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(dataset_size),
            dataset_size,
            |b, &dataset_size| {
                b.to_async(&rt).iter(|| async move {
                    let config = StorageConfig {
                        memory_cache_size: 1000,
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
                            format!("Important information about topic {} with details", i),
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

    // Benchmark: RAG with different result limits
    let mut group = c.benchmark_group("rag_result_limits");
    for limit in [1, 5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(limit), limit, |b, &_limit| {
            b.to_async(&rt).iter(|| async move {
                let config = StorageConfig {
                    memory_cache_size: 1000,
                    persist_path: None,
                    auto_cleanup: false,
                    cleanup_interval_secs: 3600,
                    enable_persistence: false,
                };
                let store = Arc::new(ContextStore::new(config).unwrap());
                let rag = RagProcessor::with_defaults(store.clone());

                // Pre-populate with contexts
                for i in 0..100 {
                    let ctx = Context::new(format!("Content {}", i), ContextDomain::Code);
                    store.store(ctx).await.unwrap();
                }

                // Perform retrieval with varying limits
                let query = RetrievalQuery::from_text("Content");

                let _result = rag.retrieve(&query).await.unwrap();
            });
        });
    }
    group.finish();

    // Benchmark: Parallel RAG processing
    c.bench_function("parallel_rag_queries", |b| {
        b.to_async(&rt).iter(|| async {
            let config = StorageConfig {
                memory_cache_size: 1000,
                persist_path: None,
                auto_cleanup: false,
                cleanup_interval_secs: 3600,
                enable_persistence: false,
            };
            let store = Arc::new(ContextStore::new(config).unwrap());
            let rag = RagProcessor::with_defaults(store.clone());

            // Pre-populate
            for i in 0..100 {
                let ctx = Context::new(format!("Content {}", i), ContextDomain::Code);
                store.store(ctx).await.unwrap();
            }

            // Multiple parallel queries
            let queries = vec![
                RetrievalQuery::from_text("Content 1"),
                RetrievalQuery::from_text("Content 2"),
                RetrievalQuery::from_text("Content 3"),
            ];

            for query in queries {
                let _result = rag.retrieve(black_box(&query)).await.unwrap();
            }
        });
    });
}

criterion_group!(benches, rag_benchmarks);
criterion_main!(benches);
