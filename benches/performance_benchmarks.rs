use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use vgo_rust::middleware::metrics::PerformanceMonitor;

fn benchmark_single_thread_operations(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());

    c.bench_function("record_request_start", |b| {
        b.iter(|| {
            let _record = monitor.record_request_start(
                black_box("/api/test"),
                black_box("GET")
            );
        })
    });

    c.bench_function("record_request_end", |b| {
        b.iter(|| {
            let record = monitor.record_request_start("/api/test", "GET");
            monitor.record_request_end(black_box(record), black_box(200));
        })
    });

    c.bench_function("get_metrics", |b| {
        b.iter(|| {
            let _metrics = monitor.get_metrics();
        })
    });
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    for thread_count in [2, 4, 8, 16].iter() {
        let monitor = Arc::new(PerformanceMonitor::new());
        
        c.bench_function(&format!("concurrent_operations_{}_threads", thread_count), |b| {
            b.iter(|| {
                let handles: Vec<_> = (0..*thread_count).map(|i| {
                    let monitor_clone = Arc::clone(&monitor);
                    thread::spawn(move || {
                        for j in 0..100 {
                            let path = format!("/api/test/{}/{}", i, j);
                            let record = monitor_clone.record_request_start(&path, "GET");
                            thread::sleep(Duration::from_nanos(1)); // 模拟处理时间
                            monitor_clone.record_request_end(record, 200);
                        }
                    })
                }).collect();
                
                for handle in handles {
                    handle.join().unwrap();
                }
            })
        });
    }
}

fn benchmark_memory_efficiency(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());
    
    c.bench_function("high_volume_requests", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let path = format!("/api/endpoint/{}", i % 10); // 重复路径
                let record = monitor.record_request_start(&path, "GET");
                monitor.record_request_end(record, 200);
            }
        })
    });
}

fn benchmark_string_pool_efficiency(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());
    
    c.bench_function("string_pool_repeated_paths", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let record = monitor.record_request_start(black_box("/api/common"), black_box("GET"));
                monitor.record_request_end(record, 200);
            }
        })
    });
    
    c.bench_function("string_pool_unique_paths", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let path = format!("/api/unique/{}", i);
                let record = monitor.record_request_start(&path, "GET");
                monitor.record_request_end(record, 200);
            }
        })
    });
}

fn benchmark_system_metrics_cache(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());
    
    c.bench_function("system_metrics_frequent_access", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let _metrics = monitor.get_metrics();
            }
        })
    });
}

fn benchmark_ring_buffer_performance(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());
    
    c.bench_function("ring_buffer_response_times", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let record = monitor.record_request_start("/api/test", "GET");
                thread::sleep(Duration::from_nanos(i % 100)); // 变化的响应时间
                monitor.record_request_end(record, 200);
            }
        })
    });
}

fn benchmark_atomic_operations(c: &mut Criterion) {
    let monitor = Arc::new(PerformanceMonitor::new());
    
    c.bench_function("atomic_operations_high_concurrency", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..8).map(|_| {
                let monitor_clone = Arc::clone(&monitor);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        let record = monitor_clone.record_request_start("/api/atomic", "GET");
                        monitor_clone.record_request_end(record, 200);
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_single_thread_operations,
    benchmark_concurrent_operations,
    benchmark_memory_efficiency,
    benchmark_string_pool_efficiency,
    benchmark_system_metrics_cache,
    benchmark_ring_buffer_performance,
    benchmark_atomic_operations
);
criterion_main!(benches);