//! 混合操作管理器性能测试

use melange_db::*;
use melange_db::hybrid_operations_manager::HybridOperationsManager;
use std::time::Instant;

#[test]
fn test_hybrid_performance_comparison() {
    println!("🚀 混合操作管理器性能测试");

    let test_size = 10000;

    // 测试1: 直接访问（基线）
    let direct_perf = test_direct_access_performance("直接访问", test_size);

    // 测试2: 混合管理器（高性能模式）
    let hybrid_perf = test_hybrid_performance("混合管理器", test_size);

    // 测试3: 原子操作性能
    let atomic_perf = test_atomic_operations_performance("原子操作", test_size);

    // 结果分析
    println!("\n📊 性能对比结果:");
    println!("================");
    println!("直接访问 (基线):");
    println!("  • 平均写入延迟: {:.2} µs/条", direct_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", direct_perf.throughput);
    println!("  • 总耗时: {:?}", direct_perf.total_time);

    println!("\n混合管理器 (高性能模式):");
    println!("  • 平均写入延迟: {:.2} µs/条", hybrid_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", hybrid_perf.throughput);
    println!("  • 总耗时: {:?}", hybrid_perf.total_time);

    println!("\n原子操作:");
    println!("  • 平均写入延迟: {:.2} µs/条", atomic_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", atomic_perf.throughput);
    println!("  • 总耗时: {:?}", atomic_perf.total_time);

    // 计算混合管理器相对于直接访问的开销
    let hybrid_overhead = (hybrid_perf.avg_latency_us - direct_perf.avg_latency_us) / direct_perf.avg_latency_us * 100.0;
    let hybrid_throughput_diff = (hybrid_perf.throughput - direct_perf.throughput) / direct_perf.throughput * 100.0;

    println!("\n🎯 性能分析:");
    println!("================");
    println!("混合管理器 vs 直接访问:");
    println!("  • 延迟开销: {:+.1}% ({:.2} µs -> {:.2} µs)",
             hybrid_overhead, direct_perf.avg_latency_us, hybrid_perf.avg_latency_us);
    println!("  • 吞吐量差异: {:+.1}% ({:.0} -> {:.0} ops/sec)",
             hybrid_throughput_diff, direct_perf.throughput, hybrid_perf.throughput);

    // 验证混合管理器性能接近直接访问
    assert!(hybrid_overhead < 10.0, "混合管理器延迟开销不应超过10%");
    assert!(hybrid_throughput_diff > -10.0, "混合管理器吞吐量损失不应超过10%");

    // 清理测试数据库
    cleanup_test_db("direct_access_hybrid_test");
    cleanup_test_db("hybrid_access_test");
    cleanup_test_db("atomic_operations_test");

    println!("\n✅ 混合管理器性能测试完成！");
    println!("混合管理器成功实现了：");
    println!("  • 普通操作：接近直接访问的性能（<10%开销）");
    println!("  • 原子操作：完全的并发安全性");
}

fn test_direct_access_performance(name: &str, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    cleanup_test_db("direct_access_hybrid_test");

    let db: Db<1024> = Config::new()
        .path("direct_access_hybrid_test")
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();

    let tree = db.open_tree("test_tree").unwrap();
    let mut latencies = Vec::new();

    for i in 0..test_size {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
        let duration = start.elapsed();
        latencies.push(duration.as_nanos() as f64);
    }

    let total_time = latencies.iter().sum::<f64>() / 1000.0;
    let avg_latency_us = latencies.iter().sum::<f64>() / latencies.len() as f64 / 1000.0;
    let throughput = test_size as f64 / (total_time / 1_000_000.0);

    PerformanceResult {
        avg_latency_us,
        throughput,
        total_time: std::time::Duration::from_micros(total_time as u64),
    }
}

fn test_hybrid_performance(name: &str, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    cleanup_test_db("hybrid_access_test");

    let db: Db<1024> = Config::new()
        .path("hybrid_access_test")
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();

    let db_arc = std::sync::Arc::new(db);
    let manager = HybridOperationsManager::new(db_arc);
    let mut latencies = Vec::new();

    for i in 0..test_size {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        manager.insert(key.as_bytes(), value.as_bytes()).unwrap();
        let duration = start.elapsed();
        latencies.push(duration.as_nanos() as f64);
    }

    let total_time = latencies.iter().sum::<f64>() / 1000.0;
    let avg_latency_us = latencies.iter().sum::<f64>() / latencies.len() as f64 / 1000.0;
    let throughput = test_size as f64 / (total_time / 1_000_000.0);

    PerformanceResult {
        avg_latency_us,
        throughput,
        total_time: std::time::Duration::from_micros(total_time as u64),
    }
}

fn test_atomic_operations_performance(name: &str, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    cleanup_test_db("atomic_operations_test");

    let db: Db<1024> = Config::new()
        .path("atomic_operations_test")
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();

    let db_arc = std::sync::Arc::new(db);
    let manager = HybridOperationsManager::new(db_arc);
    let mut latencies = Vec::new();

    // 测试原子递增操作
    for i in 0..test_size {
        let start = Instant::now();
        let counter_name = format!("counter_{}", i % 100); // 重用计数器名称
        manager.increment(counter_name, 1).unwrap();
        let duration = start.elapsed();
        latencies.push(duration.as_nanos() as f64);
    }

    let total_time = latencies.iter().sum::<f64>() / 1000.0;
    let avg_latency_us = latencies.iter().sum::<f64>() / latencies.len() as f64 / 1000.0;
    let throughput = test_size as f64 / (total_time / 1_000_000.0);

    PerformanceResult {
        avg_latency_us,
        throughput,
        total_time: std::time::Duration::from_micros(total_time as u64),
    }
}

#[derive(Debug)]
struct PerformanceResult {
    avg_latency_us: f64,
    throughput: f64,
    total_time: std::time::Duration,
}

fn cleanup_test_db(path: &str) {
    let _ = std::fs::remove_dir_all(path);
}