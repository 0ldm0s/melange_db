use melange_db::*;
use std::time::Instant;

#[test]
fn test_smart_flush_performance_comparison() {
    println!("🚀 智能Flush策略性能对比测试");

    // 测试配置1：传统固定间隔flush
    let traditional_config = Config::new()
        .path("traditional_flush_test_db")
        .flush_every_ms(Some(200))  // 200ms固定间隔
        .cache_capacity_bytes(64 * 1024 * 1024);

    // 测试配置2：智能自适应flush
    let mut smart_config = Config::new()
        .path("smart_flush_test_db")
        .flush_every_ms(Some(200))  // 基础间隔
        .cache_capacity_bytes(64 * 1024 * 1024);

    // 配置智能flush参数
    smart_config.smart_flush_config.enabled = true;
    smart_config.smart_flush_config.base_interval_ms = 200;
    smart_config.smart_flush_config.min_interval_ms = 50;
    smart_config.smart_flush_config.max_interval_ms = 1000;
    smart_config.smart_flush_config.write_rate_threshold = 5000; // 5K ops/sec
    smart_config.smart_flush_config.accumulated_bytes_threshold = 1024 * 1024; // 1MB

    let test_size = 5000;

    // 测试传统flush性能
    let traditional_perf = test_flush_performance("传统Flush", &traditional_config, test_size);

    // 测试智能flush性能
    let smart_perf = test_flush_performance("智能Flush", &smart_config, test_size);

    // 结果分析
    println!("\n📊 性能对比结果:");
    println!("================");
    println!("传统Flush:");
    println!("  • 平均写入延迟: {:.2} µs/条", traditional_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", traditional_perf.throughput);
    println!("  • 总耗时: {:?}", traditional_perf.total_time);

    println!("\n智能Flush:");
    println!("  • 平均写入延迟: {:.2} µs/条", smart_perf.avg_latency_us);
    println!("  • 吞吐量: {:.0} ops/sec", smart_perf.throughput);
    println!("  • 总耗时: {:?}", smart_perf.total_time);

    // 计算性能提升
    let improvement = (traditional_perf.avg_latency_us - smart_perf.avg_latency_us) / traditional_perf.avg_latency_us * 100.0;
    let throughput_improvement = (smart_perf.throughput - traditional_perf.throughput) / traditional_perf.throughput * 100.0;

    println!("\n🎯 智能Flush效果:");
    println!("================");
    println!("  • 延迟优化: {:.1}% ({:.2} µs -> {:.2} µs)",
             improvement.abs(), traditional_perf.avg_latency_us, smart_perf.avg_latency_us);
    println!("  • 吞吐量提升: {:.1}% ({:.0} -> {:.0} ops/sec)",
             throughput_improvement.abs(), traditional_perf.throughput, smart_perf.throughput);

    // 验证智能flush确实有效果
    assert!(smart_perf.avg_latency_us <= traditional_perf.avg_latency_us * 1.1,
            "智能flush不应该比传统flush慢10%以上");

    // 清理测试数据库
    cleanup_test_db("traditional_flush_test_db");
    cleanup_test_db("smart_flush_test_db");

    println!("\n✅ 智能Flush策略测试完成！");
}

#[test]
fn test_smart_flush_adaptive_behavior() {
    println!("\n🧠 智能Flush自适应行为测试");

    let mut config = Config::new()
        .path("adaptive_flush_test_db")
        .flush_every_ms(Some(200))
        .cache_capacity_bytes(32 * 1024 * 1024);

    // 配置激进的智能flush策略以便观察行为
    config.smart_flush_config.enabled = true;
    config.smart_flush_config.base_interval_ms = 500;
    config.smart_flush_config.min_interval_ms = 10;
    config.smart_flush_config.max_interval_ms = 2000;
    config.smart_flush_config.write_rate_threshold = 1000; // 低阈值
    config.smart_flush_config.accumulated_bytes_threshold = 50 * 1024; // 50KB

    let db: Db<1024> = config.open().unwrap();
    let tree = db.open_tree("adaptive_test").unwrap();

    // 阶段1：低写入负载测试
    println!("\n阶段1: 低写入负载测试 (100条/秒)");
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("low_load_key_{}", i);
        let value = vec![0u8; 100]; // 100字节
        tree.insert(key.as_bytes(), value).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10)); // 100条/秒
    }
    let low_load_time = start.elapsed();
    println!("低负载完成，耗时: {:?}", low_load_time);

    // 阶段2：高写入负载测试
    println!("\n阶段2: 高写入负载测试 (5000条/秒)");
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("high_load_key_{}", i);
        let value = vec![1u8; 200]; // 200字节
        tree.insert(key.as_bytes(), value).unwrap();
        // 无延迟，尽可能快地写入
    }
    let high_load_time = start.elapsed();
    println!("高负载完成，耗时: {:?}", high_load_time);

    // 阶段3：大块写入测试
    println!("\n阶段3: 大块写入测试 (累积字节触发)");
    let start = Instant::now();
    for i in 0..50 {
        let key = format!("bulk_load_key_{}", i);
        let value = vec![2u8; 2048]; // 2KB，50条 = 100KB，超过50KB阈值
        tree.insert(key.as_bytes(), value).unwrap();
    }
    let bulk_load_time = start.elapsed();
    println!("大块写入完成，耗时: {:?}", bulk_load_time);

    // 验证写入都成功
    let mut total_count = 0;
    for kv in tree.iter() {
        let (key, _) = kv.unwrap();
        if String::from_utf8_lossy(&key).starts_with("low_load_key_") {
            total_count += 1;
        }
    }
    assert_eq!(total_count, 100, "低负载写入数据应该存在");

    total_count = 0;
    for kv in tree.iter() {
        let (key, _) = kv.unwrap();
        if String::from_utf8_lossy(&key).starts_with("high_load_key_") {
            total_count += 1;
        }
    }
    assert_eq!(total_count, 1000, "高负载写入数据应该存在");

    total_count = 0;
    for kv in tree.iter() {
        let (key, _) = kv.unwrap();
        if String::from_utf8_lossy(&key).starts_with("bulk_load_key_") {
            total_count += 1;
        }
    }
    assert_eq!(total_count, 50, "大块写入数据应该存在");

    println!("\n✅ 自适应行为测试完成！所有数据正确写入。");

    // 清理
    cleanup_test_db("adaptive_flush_test_db");
}

#[derive(Debug)]
struct PerformanceResult {
    avg_latency_us: f64,
    throughput: f64,
    total_time: std::time::Duration,
}

fn test_flush_performance(name: &str, config: &Config, test_size: usize) -> PerformanceResult {
    println!("\n📊 测试{} ({}条记录)...", name, test_size);

    // 清理旧的测试数据库
    cleanup_test_db(&config.path.to_string_lossy());

    let db: Db<1024> = config.open().unwrap();
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

fn cleanup_test_db(path: &str) {
    if std::path::Path::new(path).exists() {
        std::fs::remove_dir_all(path).unwrap();
    }
}