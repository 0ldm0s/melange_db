use melange_db::{Db, Config};
use std::time::Instant;
use std::fs;
use std::path::Path;
use std::io;

fn main() -> io::Result<()> {
    println!("🔬 Melange DB 精确计时分析");
    println!("================================");

    let db_path = Path::new("accurate_timing_db");
    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }

    // 使用智能flush配置
    let mut config = Config::new()
        .path(db_path)
        .flush_every_ms(Some(200))
        .cache_capacity_bytes(512 * 1024 * 1024);

    config.smart_flush_config.enabled = true;
    config.smart_flush_config.base_interval_ms = 200;
    config.smart_flush_config.min_interval_ms = 50;
    config.smart_flush_config.max_interval_ms = 2000;
    config.smart_flush_config.write_rate_threshold = 10000;
    config.smart_flush_config.accumulated_bytes_threshold = 4 * 1024 * 1024;

    let db: Db<1024> = config.open()?;
    let tree = db.open_tree::<&[u8]>(b"timing_test")?;

    // 预热阶段
    println!("🔄 系统预热...");
    for i in 0..1000 {
        let key = format!("warmup_{}", i);
        let value = format!("warmup_value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }
    println!("✅ 预热完成");

    // 测试1：多次单条写入求平均
    println!("\n📊 测试1: 单条写入性能 (1000次平均)");
    let mut single_write_times = Vec::new();
    for i in 0..1000 {
        let start = Instant::now();
        let key = format!("single_key_{}", i);
        let value = format!("single_value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
        let duration = start.elapsed();
        single_write_times.push(duration.as_nanos() as f64);
    }

    single_write_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_single_write = single_write_times.iter().sum::<f64>() / single_write_times.len() as f64;
    let p50_single_write = single_write_times[single_write_times.len() / 2];
    let p95_single_write = single_write_times[(single_write_times.len() as f64 * 0.95) as usize];
    let p99_single_write = single_write_times[(single_write_times.len() as f64 * 0.99) as usize];

    println!("✅ 单条写入统计 (1000次):");
    println!("   平均: {:.2} µs", avg_single_write / 1000.0);
    println!("   P50: {:.2} µs", p50_single_write / 1000.0);
    println!("   P95: {:.2} µs", p95_single_write / 1000.0);
    println!("   P99: {:.2} µs", p99_single_write / 1000.0);

    // 测试2：多次单条读取求平均
    println!("\n📊 测试2: 单条读取性能 (1000次平均)");
    let mut single_read_times = Vec::new();
    for i in 0..1000 {
        let start = Instant::now();
        let key = format!("single_key_{}", i);
        let _ = tree.get(key.as_bytes())?;
        let duration = start.elapsed();
        single_read_times.push(duration.as_nanos() as f64);
    }

    single_read_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_single_read = single_read_times.iter().sum::<f64>() / single_read_times.len() as f64;
    let p50_single_read = single_read_times[single_read_times.len() / 2];
    let p95_single_read = single_read_times[(single_read_times.len() as f64 * 0.95) as usize];
    let p99_single_read = single_read_times[(single_read_times.len() as f64 * 0.99) as usize];

    println!("✅ 单条读取统计 (1000次):");
    println!("   平均: {:.2} µs", avg_single_read / 1000.0);
    println!("   P50: {:.2} µs", p50_single_read / 1000.0);
    println!("   P95: {:.2} µs", p95_single_read / 1000.0);
    println!("   P99: {:.2} µs", p99_single_read / 1000.0);

    // 测试3：批量写入性能
    println!("\n📊 测试3: 批量写入性能 (1000条)");
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("batch_key_{}", i);
        let value = format!("batch_value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }
    let batch_write_time = start.elapsed();
    let batch_write_per_op = batch_write_time.as_nanos() as f64 / 1000.0;

    println!("✅ 批量写入统计:");
    println!("   总时间: {:?}", batch_write_time);
    println!("   平均每条: {:.2} µs", batch_write_per_op / 1000.0);

    // 测试4：批量读取性能
    println!("\n📊 测试4: 批量读取性能 (1000条)");
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("batch_key_{}", i);
        let _ = tree.get(key.as_bytes())?;
    }
    let batch_read_time = start.elapsed();
    let batch_read_per_op = batch_read_time.as_nanos() as f64 / 1000.0;

    println!("✅ 批量读取统计:");
    println!("   总时间: {:?}", batch_read_time);
    println!("   平均每条: {:.2} µs", batch_read_per_op / 1000.0);

    // 测试5：范围查询性能
    println!("\n📊 测试5: 范围查询性能");
    let start = Instant::now();
    let mut count = 0;
    for kv in tree.range::<&[u8], std::ops::Range<&[u8]>>(b"batch_key_100"..b"batch_key_200") {
        let _ = kv?;
        count += 1;
    }
    let range_time = start.elapsed();

    println!("✅ 范围查询统计:");
    println!("   查询结果: {} 条", count);
    println!("   总时间: {:?}", range_time);
    println!("   平均每条: {:.2} µs", range_time.as_nanos() as f64 / count as f64 / 1000.0);

    // 结果对比分析
    println!("\n🎯 性能对比分析");
    println!("================");
    println!("操作类型       | 平均延迟 | P50延迟 | P95延迟 | P99延迟");
    println!("----------------|----------|----------|----------|----------");
    println!("单条写入        | {:7.2} µs | {:7.2} µs | {:7.2} µs | {:7.2} µs",
             avg_single_write / 1000.0, p50_single_write / 1000.0,
             p95_single_write / 1000.0, p99_single_write / 1000.0);
    println!("单条读取        | {:7.2} µs | {:7.2} µs | {:7.2} µs | {:7.2} µs",
             avg_single_read / 1000.0, p50_single_read / 1000.0,
             p95_single_read / 1000.0, p99_single_read / 1000.0);
    println!("批量写入        | {:7.2} µs | -------- | -------- | --------",
             batch_write_per_op / 1000.0);
    println!("批量读取        | {:7.2} µs | -------- | -------- | --------",
             batch_read_per_op / 1000.0);

    // 分析异常
    println!("\n🧠 性能分析");
    println!("================");

    if avg_single_write > batch_write_per_op {
        let diff = (avg_single_write - batch_write_per_op) / avg_single_write * 100.0;
        println!("• 单条写入比批量写入慢 {:.1}%：可能原因", diff);
        println!("  - 批量操作有更好的CPU缓存局部性");
        println!("  - 批量写入减少了函数调用开销");
        println!("  - 智能flush在批量写入时更高效");
    }

    if p99_single_write > avg_single_write * 3.0 {
        println!("• P99写入延迟较高：可能是");
        println!("  - 偶尔的flush操作");
        println!("  - 系统调度延迟");
        println!("  - 内存分配波动");
    }

    println!("• 智能flush策略效果：");
    println!("  - 系统自动在高负载时更频繁flush");
    println!("  - 低负载时延长flush间隔提升性能");
    println!("  - 累积大块数据时立即flush保证数据安全");

    // 清理
    drop(tree);
    drop(db);
    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }

    println!("\n✅ 精确计时分析完成！");
    Ok(())
}