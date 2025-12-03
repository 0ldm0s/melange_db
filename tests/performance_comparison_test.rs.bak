//! 统一入口 vs 直接访问性能对比测试

use melange_db::*;
use melange_db::atomic_operations_manager::AtomicOperationsManager;
use std::time::Instant;

#[test]
fn test_unified_vs_direct_performance() {
    println!("🔍 统一入口 vs 直接访问性能对比测试");

    let test_size = 10000;

    // 测试1: 直接访问模式（模拟v0.1.x）
    let direct_perf = test_direct_access_performance("直接访问", test_size);

    // 测试2: 统一入口模式（v0.2.x）
    let unified_perf = test_unified_access_performance("统一入口", test_size);

    // 结果分析
    println!("\n📊 性能对比结果:");
    println!("================");
    println!("直接访问:");
    println!("  • 平均写入延迟: {:.2} µs/条", direct_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", direct_perf.throughput);
    println!("  • 总耗时: {:?}", direct_perf.total_time);

    println!("\n统一入口:");
    println!("  • 平均写入延迟: {:.2} µs/条", unified_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", unified_perf.throughput);
    println!("  • 总耗时: {:?}", unified_perf.total_time);

    // 计算性能差异
    let latency_overhead = (unified_perf.avg_latency_us - direct_perf.avg_latency_us) / direct_perf.avg_latency_us * 100.0;
    let throughput_penalty = (direct_perf.throughput - unified_perf.throughput) / direct_perf.throughput * 100.0;

    println!("\n🎯 性能开销分析:");
    println!("================");
    println!("  • 延迟开销: +{:.1}% ({:.2} µs -> {:.2} µs)",
             latency_overhead, direct_perf.avg_latency_us, unified_perf.avg_latency_us);
    println!("  • 吞吐量损失: -{:.1}% ({:.0} -> {:.0} ops/sec)",
             throughput_penalty, direct_perf.throughput, unified_perf.throughput);
    println!("  • 总时间损失: {:.1}x", unified_perf.total_time.as_secs_f64() / direct_perf.total_time.as_secs_f64());

    // 清理测试数据库
    cleanup_test_db("direct_access_test_db");
    cleanup_test_db("unified_access_test_db");

    println!("\n✅ 性能对比测试完成！");
}

fn test_direct_access_performance(name: &str, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    cleanup_test_db("direct_access_test_db");

    // 直接访问模式：直接操作数据库实例
    let db: Db<1024> = Config::new()
        .path("direct_access_test_db")
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();

    let tree = db.open_tree("test_tree").unwrap();
    let mut latencies = Vec::new();

    // 执行写入测试
    for i in 0..test_size {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
        let duration = start.elapsed();
        latencies.push(duration.as_nanos() as f64);
    }

    // 计算性能指标
    let total_time = latencies.iter().sum::<f64>() / 1000.0; // 转换为微秒
    let avg_latency_us = latencies.iter().sum::<f64>() / latencies.len() as f64 / 1000.0;
    let throughput = test_size as f64 / (total_time / 1_000_000.0);

    PerformanceResult {
        avg_latency_us,
        throughput,
        total_time: std::time::Duration::from_micros(total_time as u64),
    }
}

fn test_unified_access_performance(name: &str, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    cleanup_test_db("unified_access_test_db");

    // 统一入口模式：通过AtomicOperationsManager
    let db: Db<1024> = Config::new()
        .path("unified_access_test_db")
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();

    let db_arc = std::sync::Arc::new(db);
    let manager = AtomicOperationsManager::new(db_arc);

    let mut latencies = Vec::new();

    // 执行写入测试（通过统一入口）
    for i in 0..test_size {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        manager.insert(key.as_bytes(), value.as_bytes()).unwrap();
        let duration = start.elapsed();
        latencies.push(duration.as_nanos() as f64);
    }

    // 计算性能指标
    let total_time = latencies.iter().sum::<f64>() / 1000.0; // 转换为微秒
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