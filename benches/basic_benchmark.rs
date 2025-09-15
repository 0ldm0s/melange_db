use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use melange_db::*;
use std::time::Duration;

fn basic_insert_benchmark(c: &mut Criterion) {
    let config = Config::new()
        .path("benchmark_db")
        .zstd_compression_level(3)
        .cache_capacity_bytes(1024 * 1024);

    // 确保测试目录干净
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }

    let mut group = c.benchmark_group("basic_operations");

    // 测试不同大小的数据插入
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, &size| {
            b.iter_batched(
                || {
                    // 每次迭代创建新的数据库
                    if std::path::Path::new("benchmark_db").exists() {
                        std::fs::remove_dir_all("benchmark_db").unwrap();
                    }
                    let db = config.clone().open::<1024>().unwrap();
                    let tree = db.open_tree("insert_test").unwrap();
                    (db, tree)
                },
                |(_db, tree)| {
                    for i in 0..size {
                        let key = format!("key_{}", i);
                        let value = format!("value_{}", i);
                        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
                    }
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();

    // 清理
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }
}

fn read_benchmark(c: &mut Criterion) {
    let config = Config::new()
        .path("benchmark_db")
        .zstd_compression_level(3)
        .cache_capacity_bytes(1024 * 1024);

    // 预先创建测试数据
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("read_test").unwrap();

    // 插入测试数据
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let mut group = c.benchmark_group("read_operations");

    group.bench_function("sequential_read", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let key = format!("key_{}", i);
                tree.get(key.as_bytes()).unwrap();
            }
        })
    });

    group.bench_function("random_read", |b| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        b.iter(|| {
            let index = rng.gen_range(0..1000);
            let key = format!("key_{}", index);
            tree.get(key.as_bytes()).unwrap();
        })
    });

    group.finish();

    // 清理
    drop(tree);
    drop(db);
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }
}

fn concurrent_insert_benchmark(c: &mut Criterion) {
    let config = Config::new()
        .path("benchmark_db")
        .zstd_compression_level(3)
        .cache_capacity_bytes(1024 * 1024);

    let mut group = c.benchmark_group("concurrent_operations");

    for thread_count in [2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::new("concurrent_insert", thread_count), thread_count, |b, &thread_count| {
            b.iter_batched(
                || {
                    // 每次迭代创建新的数据库
                    if std::path::Path::new("benchmark_db").exists() {
                        std::fs::remove_dir_all("benchmark_db").unwrap();
                    }
                    let db = config.clone().open::<1024>().unwrap();
                    db
                },
                |db| {
                    use std::sync::Arc;
                    use std::thread;

                    let db = Arc::new(db);
                    let mut handles = vec![];

                    let items_per_thread = 100 / thread_count;

                    for thread_id in 0..thread_count {
                        let db = db.clone();
                        let handle = thread::spawn(move || {
                            let tree = db.open_tree("concurrent_test").unwrap();
                            for i in 0..items_per_thread {
                                let key = format!("thread_{}_key_{}", thread_id, i);
                                let value = format!("value_{}", i);
                                tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();

    // 清理
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }
}

fn incremental_serialization_benchmark(c: &mut Criterion) {
    let config = Config::new()
        .path("benchmark_db")
        .zstd_compression_level(3)
        .incremental_serialization_threshold(50);

    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }

    let mut group = c.benchmark_group("incremental_serialization");

    group.bench_function("partial_update", |b| {
        b.iter_batched(
            || {
                if std::path::Path::new("benchmark_db").exists() {
                    std::fs::remove_dir_all("benchmark_db").unwrap();
                }
                let db = config.clone().open::<1024>().unwrap();
                let tree = db.open_tree("incremental_test").unwrap();

                // 预先插入数据
                for i in 0..100 {
                    let key = format!("key_{}", i);
                    let value = format!("initial_value_{}", i);
                    tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
                }

                (db, tree)
            },
            |(_db, tree)| {
                // 只更新部分数据
                for i in 0..10 {
                    let key = format!("key_{}", i);
                    let new_value = format!("updated_value_{}", i);
                    tree.insert(key.as_bytes(), new_value.as_bytes()).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();

    // 清理
    if std::path::Path::new("benchmark_db").exists() {
        std::fs::remove_dir_all("benchmark_db").unwrap();
    }
}

criterion_group!(
    benches,
    basic_insert_benchmark,
    read_benchmark,
    concurrent_insert_benchmark,
    incremental_serialization_benchmark
);
criterion_main!(benches);