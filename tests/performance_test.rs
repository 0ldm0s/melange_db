use melange_db::*;
use std::time::Instant;
use std::thread;
use std::sync::Arc;

/// 轻量级性能测试
///
/// 测试项目：
/// 1. 基本的插入和读取性能
/// 2. 并发写入性能
/// 3. 增量序列化效果
/// 4. 优化的flush调度器性能
///
/// 注意：这是开发设备的轻量级测试，不适合生产压力测试

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 开始 melange_db 轻量级性能测试");

    // 创建测试配置
    let config = Config::new()
        .path("perf_test_db")
        .flush_every_ms(Some(100))
        .cache_capacity_bytes(1024 * 1024) // 1MB 缓存
        .cache_warmup_strategy(CacheWarmupStrategy::None)
        .incremental_serialization_threshold(100) // 小阈值便于测试
        .zstd_compression_level(3);

    // 删除旧测试数据库
    if std::path::Path::new("perf_test_db").exists() {
        std::fs::remove_dir_all("perf_test_db")?;
    }

    // 创建数据库
    let db = config.open::<1024>()?;
    let tree = db.open_tree("performance_test")?;

    // 测试1: 基本插入和读取性能
    println!("\n📊 测试1: 基本插入和读取性能");
    let start = Instant::now();

    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }

    let insert_time = start.elapsed();
    println!("✅ 插入1000个键值对耗时: {:?}", insert_time);
    println!("   平均每条插入: {:.2}μs", insert_time.as_micros() as f64 / 1000.0);

    // 测试读取性能
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = tree.get(key.as_bytes())?;
        assert!(value.is_some());
    }

    let read_time = start.elapsed();
    println!("✅ 读取1000个键值对耗时: {:?}", read_time);
    println!("   平均每条读取: {:.2}μs", read_time.as_micros() as f64 / 1000.0);

    // 测试2: 并发写入性能
    println!("\n📊 测试2: 并发写入性能");
    let start = Instant::now();

    let db_clone = db.clone();
    let tree_name = "concurrent_test";
    let tree_clone = db_clone.open_tree(tree_name)?;

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let tree = tree_clone.clone();
            thread::spawn(move || {
                for i in 0..250 {
                    let key = format!("thread_{}_key_{}", thread_id, i);
                    let value = format!("value_from_thread_{}_{}", thread_id, i);
                    tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let concurrent_time = start.elapsed();
    println!("✅ 4线程并发插入1000个键值对耗时: {:?}", concurrent_time);
    println!("   平均每条插入: {:.2}μs", concurrent_time.as_micros() as f64 / 1000.0);

    // 验证并发写入结果
    let count = tree_clone.len()?;
    println!("   实际插入数量: {}", count);
    assert_eq!(count, 1000);

    // 测试3: 增量序列化测试
    println!("\n📊 测试3: 增量序列化效果");
    let incremental_tree = db.open_tree("incremental_test")?;

    // 先插入一些数据
    for i in 0..100 {
        let key = format!("inc_key_{}", i);
        let value = format!("inc_value_{}", i);
        incremental_tree.insert(key.as_bytes(), value.as_bytes())?;
    }

    // 更新部分数据来测试增量序列化
    let start = Instant::now();
    for i in 0..50 {
        let key = format!("inc_key_{}", i);
        let new_value = format!("updated_value_{}", i);
        incremental_tree.insert(key.as_bytes(), new_value.as_bytes())?;
    }

    let update_time = start.elapsed();
    println!("✅ 更新50个键值对耗时: {:?}", update_time);
    println!("   平均每次更新: {:.2}μs", update_time.as_micros() as f64 / 50.0);

    // 测试4: 优化的flush调度器测试
    println!("\n📊 测试4: 优化的flush调度器");
    let flush_tree = db.open_tree("flush_test")?;

    // 创建一些flush压力
    let start = Instant::now();
    for i in 0..500 {
        let key = format!("flush_key_{}", i);
        let value = vec![0u8; 1024]; // 1KB 数据
        flush_tree.insert(key.as_bytes(), value.as_slice())?;
    }

    // 强制flush
    tree.flush()?;
    flush_tree.flush()?;

    let flush_time = start.elapsed();
    println!("✅ 插入500KB数据并flush耗时: {:?}", flush_time);
    println!("   平均吞吐量: {:.2}KB/s", 500.0 / flush_time.as_secs_f64());

    // 清理
    drop(tree);
    drop(tree_clone);
    drop(incremental_tree);
    drop(flush_tree);
    drop(db);

    // 删除测试数据库
    std::fs::remove_dir_all("perf_test_db")?;

    println!("\n🎉 所有性能测试完成！");
    println!("📈 总结:");
    println!("   - 基本插入: {:.2}μs/op", insert_time.as_micros() as f64 / 1000.0);
    println!("   - 基本读取: {:.2}μs/op", read_time.as_micros() as f64 / 1000.0);
    println!("   - 并发写入: {:.2}μs/op", concurrent_time.as_micros() as f64 / 1000.0);
    println!("   - 增量更新: {:.2}μs/op", update_time.as_micros() as f64 / 50.0);
    println!("   - Flush吞吐量: {:.2}KB/s", 500.0 / flush_time.as_secs_f64());

    Ok(())
}