//! MacBook Air M1 无压缩性能示例
//!
//! 此示例展示在MacBook Air M1上使用无压缩模式的性能表现
//! 必须启用 compression-none 特性才能运行此示例
//!
//! 运行命令:
//! cargo run --example macbook_air_m1_compression_none --features compression-none --release

use melange_db::*;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 检查运行环境
    #[cfg(not(target_os = "macos"))]
    {
        println!("ℹ️  此示例专为 macOS 设计，当前操作系统不是 macOS");
        println!("ℹ️  示例将跳过实际测试，直接退出");
        return Ok(());
    }

    // 检查压缩特性
    #[cfg(not(feature = "compression-none"))]
    {
        eprintln!("❌ 错误: 此示例需要启用 compression-none 特性");
        eprintln!("❌ 请使用以下命令运行:");
        eprintln!("❌ cargo run --example macbook_air_m1_compression_none --features compression-none --release");
        return Err("未启用 compression-none 特性".into());
    }

    #[cfg(all(target_os = "macos", feature = "compression-none"))]
    {
        println!("🚀 开始 MacBook Air M1 无压缩性能测试");
        println!("💻 目标设备: MacBook Air M1 (Apple M1芯片 / 8GB内存 / macOS)");
        println!("🗜️  压缩模式: 无压缩 (CompressionAlgorithm::None)");
        println!("⚡  优势: 零CPU开销，最快读写速度，充分发挥M1性能");
        println!("🎯 M1优化: 统一内存架构 + NEON指令集 + 无压缩瓶颈");
        println!("📊 测试提示: 请使用 --release 模式运行以获得准确的性能数据");

        // 配置数据库 - 针对M1芯片优化的无压缩配置
        let mut config = Config::new()
            .path("macbook_m1_compression_none_db")
            .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
            .cache_capacity_bytes(512 * 1024 * 1024)  // 512MB缓存，利用M1统一内存架构
            .compression_algorithm(CompressionAlgorithm::None);  // 无压缩模式

        // 针对M1无压缩优化的智能flush配置
        // 由于无压缩减少了CPU开销，可以采用更激进的策略
        config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
            enabled: true,
            base_interval_ms: 25,      // 25ms基础间隔，极快响应
            min_interval_ms: 5,       // 5ms最小间隔，超低延迟
            max_interval_ms: 200,     // 200ms最大间隔，平衡性能
            write_rate_threshold: 20000, // 20K ops/sec阈值，充分利用M1性能
            accumulated_bytes_threshold: 4 * 1024 * 1024, // 4MB累积字节，更小flush单位
        };

        // 清理旧的测试数据库
        if std::path::Path::new("macbook_m1_compression_none_db").exists() {
            std::fs::remove_dir_all("macbook_m1_compression_none_db")?;
        }

        let db = config.open::<1024>()?;
        let tree = db.open_tree("compression_test")?;

        // 测试1: 单条插入性能
        println!("\n📊 测试1: 单条插入性能");
        let mut insert_times = Vec::new();

        for i in 0..5000 {
            let start = Instant::now();
            let key = format!("key_{}", i);
            let value = format!("uncompressed_m1_value_data_{}", i);
            tree.insert(key.as_bytes(), value.as_bytes())?;
            let duration = start.elapsed();
            insert_times.push(duration.as_nanos() as f64);
        }

        // 计算统计数据
        insert_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_insert = insert_times.iter().sum::<f64>() / insert_times.len() as f64;
        let p50_insert = insert_times[insert_times.len() / 2];
        let p95_insert = insert_times[(insert_times.len() as f64 * 0.95) as usize];
        let p99_insert = insert_times[(insert_times.len() as f64 * 0.99) as usize];

        println!("✅ 插入性能统计 (5000条记录 - M1无压缩):");
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
            let _ = tree.get(key.as_bytes())?;
        }

        // 测量读取性能
        for i in 0..5000 {
            let start = Instant::now();
            let key = format!("key_{}", i);
            let _ = tree.get(key.as_bytes())?;
            let duration = start.elapsed();
            read_times.push(duration.as_nanos() as f64);
        }

        // 计算统计数据
        read_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_read = read_times.iter().sum::<f64>() / read_times.len() as f64;
        let p50_read = read_times[read_times.len() / 2];
        let p95_read = read_times[(read_times.len() as f64 * 0.95) as usize];
        let p99_read = read_times[(read_times.len() as f64 * 0.99) as usize];

        println!("✅ 读取性能统计 (5000条记录 - M1无压缩):");
        println!("   平均: {:.2} µs/条", avg_read / 1000.0);
        println!("   P50: {:.2} µs/条", p50_read / 1000.0);
        println!("   P95: {:.2} µs/条", p95_read / 1000.0);
        println!("   P99: {:.2} µs/条", p99_read / 1000.0);

        // 测试3: 批量插入性能
        println!("\n📊 测试3: 批量插入性能");
        let batch_sizes = [100, 1000, 5000];

        for &batch_size in &batch_sizes {
            let mut batch_times = Vec::new();

            for _ in 0..50 {
                // 清理数据
                tree.clear()?;

                let start = Instant::now();
                for i in 0..batch_size {
                    let key = format!("batch_key_{}", i);
                    let value = format!("uncompressed_m1_batch_value_{}", i);
                    tree.insert(key.as_bytes(), value.as_bytes())?;
                }
                let duration = start.elapsed();
                batch_times.push(duration.as_nanos() as f64);
            }

            let avg_batch = batch_times.iter().sum::<f64>() / batch_times.len() as f64;
            let avg_per_op = avg_batch / batch_size as f64;

            println!("✅ 批量插入{}条: 平均 {:.2} µs/条", batch_size, avg_per_op / 1000.0);
        }

        // 测试4: 大数据值性能测试 (M1优势场景)
        println!("\n📊 测试4: 大数据值性能 (M1统一内存优势)");
        let mut large_value_times = Vec::new();
        let large_value = "x".repeat(2048); // 2KB数据

        for i in 0..1000 {
            let start = Instant::now();
            let key = format!("large_m1_key_{}", i);
            tree.insert(key.as_bytes(), large_value.as_bytes())?;
            let duration = start.elapsed();
            large_value_times.push(duration.as_nanos() as f64);
        }

        let avg_large = large_value_times.iter().sum::<f64>() / large_value_times.len() as f64;
        println!("✅ 大数据值插入 (2KB): 平均 {:.2} µs/条", avg_large / 1000.0);

        // 测试5: 并发性能测试 (M1多核优势)
        println!("\n📊 测试5: 并发写入性能 (M1 8核优势)");
        use std::sync::Arc;
        use std::thread;

        let db_clone = Arc::new(db.clone());
        let mut handles = vec![];

        let start = Instant::now();

        // 利用M1的8核心设计
        for thread_id in 0..8 {
            let db_clone = db_clone.clone();
            let handle = thread::spawn(move || {
                let tree = db_clone.open_tree("concurrent_test")?;
                for i in 0..1000 {
                    let key = format!("m1_concurrent_key_{}_{}", thread_id, i);
                    let value = format!("uncompressed_m1_concurrent_value_{}_{}", thread_id, i);
                    tree.insert(key.as_bytes(), value.as_bytes())?;
                }
                Ok::<(), std::io::Error>(())
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap()?;
        }

        let concurrent_duration = start.elapsed();
        let concurrent_ops = 8 * 1000;
        let avg_concurrent = concurrent_duration.as_nanos() as f64 / concurrent_ops as f64;

        println!("✅ 并发写入性能 (8线程 - M1无压缩):");
        println!("   总耗时: {:?}", concurrent_duration);
        println!("   平均: {:.2} µs/条", avg_concurrent / 1000.0);
        println!("   吞吐量: {:.0} ops/sec", concurrent_ops as f64 / concurrent_duration.as_secs_f64());

        // 清理
        drop(tree);
        drop(db);
        std::fs::remove_dir_all("macbook_m1_compression_none_db")?;

        println!("\n🎉 MacBook Air M1 无压缩性能测试完成！");
        println!("📈 设备配置: MacBook Air M1 - Apple M1芯片 (8核), 8GB统一内存");
        println!("🗜️  压缩配置: CompressionAlgorithm::None");
        println!("📊 M1无压缩模式性能特点:");
        println!("   - 写入: {:.1} µs/条 (零压缩开销 + M1高性能)", avg_insert / 1000.0);
        println!("   - 读取: {:.1} µs/条 (零解压缩开销 + 统一内存)", avg_read / 1000.0);
        println!("   - 并发: {:.1} µs/条 (8核心优势)", avg_concurrent / 1000.0);
        println!("   - 大数据: {:.1} µs/条 (统一内存架构)", avg_large / 1000.0);

        println!("\n🎯 M1无压缩模式优势:");
        println!("   ✅ 充分发挥M1芯片的极致性能");
        println!("   ✅ 统一内存架构减少数据拷贝开销");
        println!("   ✅ NEON指令集加速数据处理");
        println!("   ✅ 8核心设计提供卓越并发性能");
        println!("   ✅ 零压缩延迟，适合实时应用");

        println!("\n🔍 M1性能评估:");
        let m1_excellent_write = 1.5;
        let m1_excellent_read = 0.8;

        if avg_insert / 1000.0 <= m1_excellent_write && avg_read / 1000.0 <= m1_excellent_read {
            println!("✅ M1无压缩模式性能表现卓越，充分发挥了Apple Silicon的优势！");
        } else if avg_insert / 1000.0 <= m1_excellent_write * 1.5 && avg_read / 1000.0 <= m1_excellent_read * 1.5 {
            println!("✅ M1无压缩模式性能表现优秀，适合高性能应用场景");
        } else {
            println!("⚠️  M1无压缩模式性能低于预期，建议检查系统状态");
        }

        println!("\n💡 M1无压缩模式适用场景:");
        println!("   - 实时数据处理系统");
        println!("   - 高频交易应用");
        println!("   - 游戏和交互式应用");
        println!("   - 科学计算和数据分析");
        println!("   - 需要极致性能的任何场景");
        println!("   - 存储空间充足的M1设备");

        println!("\n🚀 M1优化总结:");
        println!("   - 统一内存架构: CPU和GPU共享内存，减少数据拷贝");
        println!("   - NEON指令集: 高效的SIMD数据处理");
        println!("   - 8核心设计: 4性能核+4能效核，智能调度");
        println!("   - 无压缩瓶颈: 消除压缩算法的性能限制");
        println!("   - Apple Silicon优化: 专为macOS优化的硬件架构");
    }

    Ok(())
}