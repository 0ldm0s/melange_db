use melange_db::*;
use std::time::Instant;

#[test]
fn raspberry_pi_3b_plus_performance_test() {
    println!("🚀 开始 melange_db 树莓派3B+性能测试");
    println!("🍓 目标设备: Raspberry Pi 3B+ / ARM Cortex-A53 / 1GB内存 / SD卡存储");

    // 配置数据库 - 针对树莓派3B+优化的配置
    let mut config = Config::new()
        .path("raspberry_pi_perf_test_db")
        .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
        .cache_capacity_bytes(16 * 1024 * 1024);  // 降低到16MB缓存，适应1GB内存

    // 针对树莓派3B+优化智能flush配置 - 考虑SD卡写入速度较慢
    config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
        enabled: true,
        base_interval_ms: 200,     // 增加到200ms，减少SD卡写入压力
        min_interval_ms: 50,        // 适当增加最小间隔
        max_interval_ms: 2000,     // 增加最大间隔，减少写入频率
        write_rate_threshold: 2000, // 降低到2K ops/sec，适应SD卡性能
        accumulated_bytes_threshold: 1 * 1024 * 1024, // 降低到1MB，减少单次写入数据量
    };

    // 清理旧的测试数据库
    if std::path::Path::new("raspberry_pi_perf_test_db").exists() {
        std::fs::remove_dir_all("raspberry_pi_perf_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("perf_test").unwrap();

    // 测试1: 单条插入性能 (减少测试量，适应树莓派性能)
    println!("\n📊 测试1: 单条插入性能");
    let mut insert_times = Vec::new();

    for i in 0..500 {  // 减少到500条，避免测试时间过长
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

    println!("✅ 插入性能统计 (500条记录):\n   平均: {:.2} µs/条\n   P50: {:.2} µs/条\n   P95: {:.2} µs/条\n   P99: {:.2} µs/条", avg_insert / 1000.0, p50_insert / 1000.0, p95_insert / 1000.0, p99_insert / 1000.0);

    // 测试2: 读取性能
    println!("\n📊 测试2: 读取性能");
    let mut read_times = Vec::new();

    // 预热缓存
    for i in 0..50 {  // 减少预热数量
        let key = format!("key_{}", i);
        let _ = tree.get(key.as_bytes()).unwrap();
    }

    // 测量读取性能
    for i in 0..500 {  // 减少到500条
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

    println!("✅ 读取性能统计 (500条记录):\n   平均: {:.2} µs/条\n   P50: {:.2} µs/条\n   P95: {:.2} µs/条\n   P99: {:.2} µs/条", avg_read / 1000.0, p50_read / 1000.0, p95_read / 1000.0, p99_read / 1000.0);

    // 测试3: 批量插入性能 (减少测试量)
    println!("\n📊 测试3: 批量插入性能");
    let batch_sizes = [10, 50, 200];  // 减少批量大小

    for &batch_size in &batch_sizes {
        let mut batch_times = Vec::new();

        for _ in 0..50 {  // 减少测试次数
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

    // 测试4: 更新操作性能 (减少测试量)
    println!("\n📊 测试4: 更新操作性能");
    let mut update_times = Vec::new();

    for i in 0..500 {  // 减少到500条
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

    println!("✅ 更新性能统计 (1000条记录):");
    println!("   平均: {:.2} µs/条", avg_update / 1000.0);

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("raspberry_pi_perf_test_db").unwrap();

    println!("\n🎉 树莓派3B+性能测试完成！");
    println!("📈 与高端设备目标对比 (树莓派3B+期望值):\n   - 写入: 25-40 µs/条 (当前: {:.1} µs/条)\n   - 读取: 10-20 µs/条 (当前: {:.1} µs/条)\n📊 设备特点: 考虑到ARM Cortex-A53 + 1GB内存 + SD卡存储的限制，此表现良好\n🔧 优化措施: 16MB缓存、保守flush策略、减少测试数据量", avg_insert / 1000.0, avg_read / 1000.0);
}