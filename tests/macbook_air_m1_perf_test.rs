use melange_db::*;
use std::time::Instant;

#[test]
fn macbook_air_m1_performance_test() {
    println!("🚀 开始 melange_db MacBook Air M1 性能测试");
    println!("🖥️  目标设备: Apple M1芯片 / 8GB内存 / ARM64 NEON指令集");
    println!("⚠️  重要提示: 请使用 --release 模式运行以获得准确的性能数据");
    println!("   命令: cargo test --release macbook_air_m1_performance_test");

    // 配置数据库 - 针对M1芯片优化的配置
    let mut config = Config::new()
        .path("macbook_m1_perf_test_db")
        .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
        .cache_capacity_bytes(512 * 1024 * 1024);  // 512MB缓存，利用M1的统一内存架构

    // 针对M1芯片优化智能flush配置
    config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
        enabled: true,
        base_interval_ms: 50,      // 降低到50ms，利用M1的高性能
        min_interval_ms: 10,       // 更小最小间隔，提高响应性
        max_interval_ms: 800,      // 较低的最大间隔，保证数据安全
        write_rate_threshold: 15000, // 提高到15K ops/sec，M1可以处理更高负载
        accumulated_bytes_threshold: 8 * 1024 * 1024, // 8MB，平衡性能和持久化
    };

    // 清理旧的测试数据库
    if std::path::Path::new("macbook_m1_perf_test_db").exists() {
        std::fs::remove_dir_all("macbook_m1_perf_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("perf_test").unwrap();

    // 测试1: 单条插入性能
    println!("\n📊 测试1: 单条插入性能");
    let mut insert_times = Vec::new();

    for i in 0..5000 {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
        let duration = start.elapsed();
        insert_times.push(duration.as_nanos() as f64);
    }

    // 计算统计数据
    insert_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_insert = insert_times.iter().sum::<f64>() / insert_times.len() as f64;
    let p50_insert = insert_times[insert_times.len() / 2];
    let p95_insert = insert_times[(insert_times.len() as f64 * 0.95) as usize];
    let p99_insert = insert_times[(insert_times.len() as f64 * 0.99) as usize];

    println!("✅ 插入性能统计 (5000条记录):");
    println!("   平均: {:.2} µs/条", avg_insert / 1000.0);
    println!("   P50: {:.2} µs/条", p50_insert / 1000.0);
    println!("   P95: {:.2} µs/条", p95_insert / 1000.0);
    println!("   P99: {:.2} µs/条", p99_insert / 1000.0);

    // 测试2: 读取性能
    println!("\n📊 测试2: 读取性能");
    let mut read_times = Vec::new();

    // 预热缓存
    for i in 0..500 {
        let key = format!("key_{}", i);
        let _ = tree.get(key.as_bytes()).unwrap();
    }

    // 测量读取性能
    for i in 0..5000 {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let _ = tree.get(key.as_bytes()).unwrap();
        let duration = start.elapsed();
        read_times.push(duration.as_nanos() as f64);
    }

    // 计算统计数据
    read_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_read = read_times.iter().sum::<f64>() / read_times.len() as f64;
    let p50_read = read_times[read_times.len() / 2];
    let p95_read = read_times[(read_times.len() as f64 * 0.95) as usize];
    let p99_read = read_times[(read_times.len() as f64 * 0.99) as usize];

    println!("✅ 读取性能统计 (5000条记录):");
    println!("   平均: {:.2} µs/条", avg_read / 1000.0);
    println!("   P50: {:.2} µs/条", p50_read / 1000.0);
    println!("   P95: {:.2} µs/条", p95_read / 1000.0);
    println!("   P99: {:.2} µs/条", p99_read / 1000.0);

    // 测试3: 批量插入性能
    println!("\n📊 测试3: 批量插入性能");
    let batch_sizes = [50, 500, 5000];

    for &batch_size in &batch_sizes {
        let mut batch_times = Vec::new();

        for _ in 0..50 {
            // 清理数据
            tree.clear().unwrap();

            let start = Instant::now();
            for i in 0..batch_size {
                let key = format!("batch_key_{}", i);
                let value = format!("batch_value_{}", i);
                tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
            }
            let duration = start.elapsed();
            batch_times.push(duration.as_nanos() as f64);
        }

        let avg_batch = batch_times.iter().sum::<f64>() / batch_times.len() as f64;
        let avg_per_op = avg_batch / batch_size as f64;

        println!("✅ 批量插入{}条: 平均 {:.2} µs/条", batch_size, avg_per_op / 1000.0);
    }

    // 测试4: 更新操作性能
    println!("\n📊 测试4: 更新操作性能");
    let mut update_times = Vec::new();

    for i in 0..5000 {
        let start = Instant::now();
        let key = format!("key_{}", i);
        let new_value = format!("updated_value_{}", i);
        tree.insert(key.as_bytes(), new_value.as_bytes()).unwrap();
        let duration = start.elapsed();
        update_times.push(duration.as_nanos() as f64);
    }

    // 计算统计数据
    update_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_update = update_times.iter().sum::<f64>() / update_times.len() as f64;

    println!("✅ 更新性能统计 (5000条记录):");
    println!("   平均: {:.2} µs/条", avg_update / 1000.0);

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("macbook_m1_perf_test_db").unwrap();

    println!("\n🎉 MacBook Air M1 性能测试完成！");
    println!("📈 与高端设备目标对比 (M1芯片期望值):");
    println!("   - 写入: 1-3 µs/条 (当前: {:.1} µs/条)", avg_insert / 1000.0);
    println!("   - 读取: 0.5-2 µs/条 (当前: {:.1} µs/条)", avg_read / 1000.0);
    println!("📊 M1芯片优化特点:");
    println!("   - 统一内存架构减少数据拷贝");
    println!("   - NEON指令集加速数据处理");
    println!("   - 8核心设计提供优秀并发性能");
    println!("   - 512MB缓存充分利用可用内存");
}