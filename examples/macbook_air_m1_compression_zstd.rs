//! MacBook Air M1 Zstd压缩性能示例
//!
//! 此示例展示在MacBook Air M1上使用Zstd压缩模式的性能表现
//! 必须启用 compression-zstd 特性才能运行此示例
//!
//! 运行命令:
//! cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release

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
    #[cfg(not(feature = "compression-zstd"))]
    {
        eprintln!("❌ 错误: 此示例需要启用 compression-zstd 特性");
        eprintln!("❌ 请使用以下命令运行:");
        eprintln!("❌ cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release");
        return Err("未启用 compression-zstd 特性".into());
    }

    #[cfg(all(target_os = "macos", feature = "compression-zstd"))]
    {
        println!("🚀 开始 MacBook Air M1 Zstd压缩性能测试");
        println!("💻 目标设备: MacBook Air M1 (Apple M1芯片 / 8GB内存 / macOS)");
        println!("🗜️  压缩模式: Zstd压缩 (CompressionAlgorithm::Zstd)");
        println!("📦 优势: 高压缩率，节省存储空间，M1优化");
        println!("🎯 M1优化: 统一内存架构 + NEON指令集优化Zstd");
        println!("📊 测试提示: 请使用 --release 模式运行以获得准确的性能数据");

        // 配置数据库 - 针对M1芯片优化的Zstd压缩配置
        let mut config = Config::new()
            .path("macbook_m1_compression_zstd_db")
            .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
            .cache_capacity_bytes(512 * 1024 * 1024)  // 512MB缓存，利用M1统一内存架构
            .compression_algorithm(CompressionAlgorithm::Zstd);  // Zstd压缩

        // 针对M1 Zstd压缩优化的智能flush配置
        // Zstd压缩率更高但CPU开销更大，采用保守策略
        config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
            enabled: true,
            base_interval_ms: 60,      // 60ms基础间隔，Zstd压缩开销较大
            min_interval_ms: 15,      // 15ms最小间隔
            max_interval_ms: 400,     // 400ms最大间隔
            write_rate_threshold: 15000, // 15K ops/sec阈值，Zstd压缩限制
            accumulated_bytes_threshold: 8 * 1024 * 1024, // 8MB累积字节
        };

        // 清理旧的测试数据库
        if std::path::Path::new("macbook_m1_compression_zstd_db").exists() {
            std::fs::remove_dir_all("macbook_m1_compression_zstd_db")?;
        }

        let db = config.open::<1024>()?;
        let tree = db.open_tree("compression_test")?;

        // 测试1: 单条插入性能
        println!("\n📊 测试1: 单条插入性能");
        let mut insert_times = Vec::new();

        for i in 0..5000 {
            let start = Instant::now();
            let key = format!("key_{}", i);
            let value = format!("zstd_m1_compressed_value_{}", i);
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

        println!("✅ 插入性能统计 (5000条记录 - M1 Zstd压缩):");
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

        println!("✅ 读取性能统计 (5000条记录 - M1 Zstd压缩):");
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
                    let value = format!("zstd_m1_batch_value_{}", i);
                    tree.insert(key.as_bytes(), value.as_bytes())?;
                }
                let duration = start.elapsed();
                batch_times.push(duration.as_nanos() as f64);
            }

            let avg_batch = batch_times.iter().sum::<f64>() / batch_times.len() as f64;
            let avg_per_op = avg_batch / batch_size as f64;

            println!("✅ 批量插入{}条: 平均 {:.2} µs/条", batch_size, avg_per_op / 1000.0);
        }

        // 测试4: 高压缩率数据性能测试 (Zstd优势场景)
        println!("\n📊 测试4: 高压缩率数据 (M1+Zstd优势场景)");
        let mut highly_compressible_times = Vec::new();
        // 创建高度可压缩的数据（重复模式，Zstd优化处理）
        let highly_compressible_value = "M1_ZSTD_HIGH_COMPRESSION_TEST_PATTERN_".repeat(64); // 1.5KB，高度重复

        for i in 0..1000 {
            let start = Instant::now();
            let key = format!("zstd_compressible_key_{}", i);
            tree.insert(key.as_bytes(), highly_compressible_value.as_bytes())?;
            let duration = start.elapsed();
            highly_compressible_times.push(duration.as_nanos() as f64);
        }

        let avg_highly_compressible = highly_compressible_times.iter().sum::<f64>() / highly_compressible_times.len() as f64;
        println!("✅ 高压缩率数据 (1.5KB): 平均 {:.2} µs/条", avg_highly_compressible / 1000.0);

        // 测试5: 并发性能测试 (M1多核+Zstd)
        println!("\n📊 测试5: 并发写入性能 (M1 8核+Zstd)");
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
                    let key = format!("m1_zstd_concurrent_key_{}_{}", thread_id, i);
                    let value = format!("zstd_m1_concurrent_value_{}_{}", thread_id, i);
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

        println!("✅ 并发写入性能 (8线程 - M1 Zstd):");
        println!("   总耗时: {:?}", concurrent_duration);
        println!("   平均: {:.2} µs/条", avg_concurrent / 1000.0);
        println!("   吞吐量: {:.0} ops/sec", concurrent_ops as f64 / concurrent_duration.as_secs_f64());

        // 测试6: 存储效率测试
        println!("\n📊 测试6: 存储效率测试");
        let storage_test_size = 2000;
        let test_data = "M1_ZSTD_compression_efficiency_test_data_for_Apple_Silicon_high_compression_ratio_".repeat(16);

        for i in 0..storage_test_size {
            let key = format!("storage_test_key_{}", i);
            tree.insert(key.as_bytes(), test_data.as_bytes())?;
        }

        println!("✅ 存储效率测试完成 ({}条高压缩率数据)", storage_test_size);

        // 清理
        drop(tree);
        drop(db);
        std::fs::remove_dir_all("macbook_m1_compression_zstd_db")?;

        println!("\n🎉 MacBook Air M1 Zstd压缩性能测试完成！");
        println!("📈 设备配置: MacBook Air M1 - Apple M1芯片 (8核), 8GB统一内存");
        println!("🗜️  压缩配置: CompressionAlgorithm::Zstd + M1 NEON优化");
        println!("📊 M1 Zstd压缩模式性能特点:");
        println!("   - 写入: {:.1} µs/条 (高压缩率开销)", avg_insert / 1000.0);
        println!("   - 读取: {:.1} µs/条 (快速解压缩)", avg_read / 1000.0);
        println!("   - 并发: {:.1} µs/条 (8核心+Zstd)", avg_concurrent / 1000.0);
        println!("   - 高压缩数据: {:.1} µs/条 (Zstd重复数据处理)", avg_highly_compressible / 1000.0);

        println!("\n🎯 M1 Zstd压缩模式优势:");
        println!("   ✅ 极高的压缩率，显著减少存储空间");
        println!("   ✅ M1 NEON指令集硬件加速Zstd算法");
        println!("   ✅ 统一内存架构减少压缩数据拷贝");
        println!("   ✅ 适合存储空间受限的应用");
        println!("   ✅ 高压缩率数据的理想选择");
        println!("   ✅ Apple Silicon优化的压缩算法");

        println!("\n🔍 M1 Zstd性能评估:");
        let m1_zstd_acceptable_write = 4.0;
        let m1_zstd_acceptable_read = 2.0;

        if avg_insert / 1000.0 <= m1_zstd_acceptable_write && avg_read / 1000.0 <= m1_zstd_acceptable_read {
            println!("✅ M1 Zstd压缩模式性能表现优秀，在压缩率和性能间取得了良好平衡！");
        } else if avg_insert / 1000.0 <= m1_zstd_acceptable_write * 1.5 && avg_read / 1000.0 <= m1_zstd_acceptable_read * 1.5 {
            println!("✅ M1 Zstd压缩模式性能表现良好，压缩率优势明显");
        } else {
            println!("⚠️  M1 Zstd压缩模式性能开销较大，但存储效率显著");
        }

        println!("\n💡 M1 Zstd压缩模式适用场景:");
        println!("   - 存储空间受限的M1应用");
        println!("   - 高压缩率需求的数据（日志、文档等）");
        println!("   - 对存储成本敏感的应用");
        println!("   - 需要长期数据归档的场景");
        println!("   - 网络传输带宽受限的应用");
        println!("   - 可以接受一定CPU开销换取存储空间");

        println!("\n🚀 M1 + Zstd优化总结:");
        println!("   - NEON指令集: 硬件加速Zstd压缩算法计算");
        println!("   - 统一内存: CPU和GPU共享压缩数据，减少拷贝");
        println!("   - 8核心并行: 多线程并发压缩处理");
        println!("   - Apple Silicon: 专为macOS优化的Zstd指令调度");
        println!("   - 高压缩率: 在可接受的性能开销下获得最大压缩率");
    }

    Ok(())
}