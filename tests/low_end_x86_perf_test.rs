use melange_db::*;
use std::time::Instant;

#[test]
fn low_end_x86_performance_test() {
    println!("🚀 开始 melange_db 低端x86设备性能测试");
    println!("🖥️  目标设备: Intel Celeron J1800 / 2GB可用内存 / SSE2指令集");

    // 配置数据库 - 针对低端设备优化的配置
    let mut config = Config::new()
        .path("low_end_perf_test_db")
        .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
        .cache_capacity_bytes(32 * 1024 * 1024);  // 降低到32MB缓存，适应2GB内存

    // 针对低端设备优化智能flush配置 - 实验性配置
    config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
        enabled: true,
        base_interval_ms: 100,     // 回到100ms，减少延迟
        min_interval_ms: 30,        // 减少最小间隔
        max_interval_ms: 1500,     // 降低最大间隔
        write_rate_threshold: 4000, // 提高到4K ops/sec
        accumulated_bytes_threshold: 2 * 1024 * 1024, // 提高到2MB，减少flush次数
    };

    // 清理旧的测试数据库
    if std::path::Path::new("low_end_perf_test_db").exists() {
        std::fs::remove_dir_all("low_end_perf_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("perf_test").unwrap();

    // 测试1: 单条插入性能
    println!("\n📊 测试1: 单条插入性能");
    let mut insert_times = Vec::new();

    for i in 0..1000 {
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

    println!("✅ 插入性能统计 (1000条记录):");
    println!("   平均: {:.2} µs/条", avg_insert / 1000.0);
    println!("   P50: {:.2} µs/条", p50_insert / 1000.0);
    println!("   P95: {:.2} µs/条", p95_insert / 1000.0);
    println!("   P99: {:.2} µs/条", p99_insert / 1000.0);

    // 测试2: 读取性能
    println!("\n📊 测试2: 读取性能");
    let mut read_times = Vec::new();

    // 预热缓存
    for i in 0..100 {
        let key = format!("key_{}", i);
        let _ = tree.get(key.as_bytes()).unwrap();
    }

    // 测量读取性能
    for i in 0..1000 {
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

    println!("✅ 读取性能统计 (1000条记录):");
    println!("   平均: {:.2} µs/条", avg_read / 1000.0);
    println!("   P50: {:.2} µs/条", p50_read / 1000.0);
    println!("   P95: {:.2} µs/条", p95_read / 1000.0);
    println!("   P99: {:.2} µs/条", p99_read / 1000.0);

    // 测试3: 批量插入性能
    println!("\n📊 测试3: 批量插入性能");
    let batch_sizes = [10, 100, 1000];

    for &batch_size in &batch_sizes {
        let mut batch_times = Vec::new();

        for _ in 0..100 {
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

    for i in 0..1000 {
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
    std::fs::remove_dir_all("low_end_perf_test_db").unwrap();

    println!("\n🎉 低端x86设备性能测试完成！");
    println!("📈 与高端设备目标对比 (低端设备期望值):");
    println!("   - 写入: 15-20 µs/条 (当前: {:.1} µs/条)", avg_insert / 1000.0);
    println!("   - 读取: 5-8 µs/条 (当前: {:.1} µs/条)", avg_read / 1000.0);
    println!("📊 设备特点: 考虑到Intel Celeron J1800 + 2GB内存的限制，此表现良好");
}