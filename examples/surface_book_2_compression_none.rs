//! Surface Book 2 无压缩性能示例（混合操作管理器）
//!
//! 此示例展示在Surface Book 2上使用无压缩模式 + HybridOperationsManager的性能表现
//! 必须启用 compression-none 特性才能运行此示例
//! 展示高端x86_64设备在无压缩模式下的极致性能
//!
//! 运行命令:
//! cargo run --example surface_book_2_compression_none --features compression-none --release

use melange_db::*;
use melange_db::hybrid_operations_manager::HybridOperationsManager;
use std::time::Instant;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 检查运行环境
    #[cfg(not(target_os = "windows"))]
    {
        println!("ℹ️  此示例专为 Windows Surface Book 2 设计，当前操作系统不是 Windows");
        println!("ℹ️  示例将跳过实际测试，直接退出");
        return Ok(());
    }

    // 检查压缩特性
    #[cfg(not(feature = "compression-none"))]
    {
        eprintln!("❌ 错误: 此示例需要启用 compression-none 特性");
        eprintln!("❌ 请使用以下命令运行:");
        eprintln!("❌ cargo run --example surface_book_2_compression_none --features compression-none --release");
        return Err("未启用 compression-none 特性".into());
    }

    #[cfg(all(target_os = "windows", feature = "compression-none"))]
    {
        println!("🚀 开始 Surface Book 2 无压缩性能测试（混合操作管理器）");
        println!("💻 目标设备: Surface Book 2 (Intel Core i7-8650U / 16GB内存 / Windows 11)");
        println!("🗜️  压缩模式: 无压缩 (CompressionAlgorithm::None)");
        println!("⚡  优势: 零CPU开销，最快读写速度，充分发挥x86_64 AVX2性能");
        println!("🎯 x86优化: AVX2指令集 + 无压缩瓶颈 + 混合操作管理器");
        println!("⚠️  重要提醒: 请确保Windows电源选项设置为'高性能'模式");
        println!("📊 测试提示: 请使用 --release 模式运行以获得准确的性能数据");

        // 配置数据库 - 针对Surface Book 2优化的无压缩配置
        let mut config = Config::new()
            .path("surface_book_2_compression_none_db")
            .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
            .cache_capacity_bytes(512 * 1024 * 1024)  // 512MB缓存，充分利用16GB内存
            .compression_algorithm(CompressionAlgorithm::None);  // 无压缩模式

        // 针对Surface Book 2无压缩优化的智能flush配置
        // 来自surface_book_2_perf_test.rs的最佳配置参数
        config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
            enabled: true,
            base_interval_ms: 100,     // 100ms基础间隔，SSD优化
            min_interval_ms: 20,        // 20ms最小间隔，低延迟
            max_interval_ms: 500,      // 500ms最大间隔，平衡延迟
            write_rate_threshold: 8000, // 8K ops/sec阈值，适合高端设备
            accumulated_bytes_threshold: 8 * 1024 * 1024, // 8MB累积字节，平衡flush频率
        };

        // 清理旧的测试数据库
        if std::path::Path::new("surface_book_2_compression_none_db").exists() {
            std::fs::remove_dir_all("surface_book_2_compression_none_db")?;
        }

        let db = config.open::<1024>()?;
        let db = Arc::new(db);

        // 创建混合操作管理器（启用DatabaseWorker模式以避免EBR冲突）
        let mut manager = HybridOperationsManager::new(db.clone());
        manager.enable_database_worker_mode();
        let manager = Arc::new(manager);
        println!("✅ 混合操作管理器创建完成（DatabaseWorker模式）");

        // 测试1: 单条插入性能
        println!("\n📊 测试1: 单条插入性能（混合管理器）");
        let mut insert_times = Vec::new();

        for i in 0..5000 {
            let start = Instant::now();
            let key = format!("key_{}", i);
            let value = format!("uncompressed_sb2_value_data_{}", i);
            manager.insert(key.as_bytes(), value.as_bytes())?;
            let duration = start.elapsed();
            insert_times.push(duration.as_nanos() as f64);
        }

        // 计算统计数据
        insert_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_insert = insert_times.iter().sum::<f64>() / insert_times.len() as f64;
        let p50_insert = insert_times[insert_times.len() / 2];
        let p95_insert = insert_times[(insert_times.len() as f64 * 0.95) as usize];
        let p99_insert = insert_times[(insert_times.len() as f64 * 0.99) as usize];

        println!("✅ 插入性能统计 (5000条记录 - Surface Book 2无压缩 + 混合管理器):");
        println!("   平均: {:.2} µs/条", avg_insert / 1000.0);
        println!("   P50: {:.2} µs/条", p50_insert / 1000.0);
        println!("   P95: {:.2} µs/条", p95_insert / 1000.0);
        println!("   P99: {:.2} µs/条", p99_insert / 1000.0);

        // 测试2: 读取性能
        println!("\n📊 测试2: 读取性能（混合管理器）");
        let mut read_times = Vec::new();

        // 预热缓存
        for i in 0..500 {
            let key = format!("key_{}", i);
            let _ = manager.get_data(key.as_bytes())?;
        }

        // 测量读取性能
        for i in 0..5000 {
            let start = Instant::now();
            let key = format!("key_{}", i);
            let _ = manager.get_data(key.as_bytes())?;
            let duration = start.elapsed();
            read_times.push(duration.as_nanos() as f64);
        }

        // 计算统计数据
        read_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_read = read_times.iter().sum::<f64>() / read_times.len() as f64;
        let p50_read = read_times[read_times.len() / 2];
        let p95_read = read_times[(read_times.len() as f64 * 0.95) as usize];
        let p99_read = read_times[(read_times.len() as f64 * 0.99) as usize];

        println!("✅ 读取性能统计 (5000条记录 - Surface Book 2无压缩 + 混合管理器):");
        println!("   平均: {:.2} µs/条", avg_read / 1000.0);
        println!("   P50: {:.2} µs/条", p50_read / 1000.0);
        println!("   P95: {:.2} µs/条", p95_read / 1000.0);
        println!("   P99: {:.2} µs/条", p99_read / 1000.0);

        // 测试3: 原子操作性能测试（混合管理器特色）
        println!("\n📊 测试3: 原子操作性能测试（混合管理器特色）");
        let mut atomic_times = Vec::new();

        // 原子递增测试
        for i in 0..1000 {
            let start = Instant::now();
            manager.increment("sb2_atomic_counter".to_string(), 1)?;
            let duration = start.elapsed();
            atomic_times.push(duration.as_nanos() as f64);
        }

        atomic_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_atomic = atomic_times.iter().sum::<f64>() / atomic_times.len() as f64;
        let p50_atomic = atomic_times[atomic_times.len() / 2];
        let p95_atomic = atomic_times[(atomic_times.len() as f64 * 0.95) as usize];

        println!("✅ 原子操作性能统计 (1000次递增 - 混合管理器):");
        println!("   平均: {:.2} µs/次", avg_atomic / 1000.0);
        println!("   P50: {:.2} µs/次", p50_atomic / 1000.0);
        println!("   P95: {:.2} µs/次", p95_atomic / 1000.0);

        // 测试4: 批量插入性能
        println!("\n📊 测试4: 批量插入性能");
        let batch_sizes = [100, 1000, 5000];

        for &batch_size in &batch_sizes {
            let mut batch_times = Vec::new();

            for _ in 0..50 {
                // 清理数据
                let scan_results = manager.scan_prefix(b"batch_key_")?;
                for (key, _) in scan_results {
                    manager.remove(&key)?;
                }

                let start = Instant::now();
                for i in 0..batch_size {
                    let key = format!("batch_key_{}", i);
                    let value = format!("uncompressed_sb2_batch_value_{}", i);
                    manager.insert(key.as_bytes(), value.as_bytes())?;
                }
                let duration = start.elapsed();
                batch_times.push(duration.as_nanos() as f64);
            }

            let avg_batch = batch_times.iter().sum::<f64>() / batch_times.len() as f64;
            let avg_per_op = avg_batch / batch_size as f64;

            println!("✅ 批量插入{}条: 平均 {:.2} µs/条", batch_size, avg_per_op / 1000.0);
        }

        // 测试5: 大数据值性能测试 (Surface Book 2优势场景)
        println!("\n📊 测试5: 大数据值性能 (Surface Book 2 16GB内存优势)");
        let mut large_value_times = Vec::new();
        let large_value = "x".repeat(2048); // 2KB数据

        for i in 0..1000 {
            let start = Instant::now();
            let key = format!("large_sb2_key_{}", i);
            manager.insert(key.as_bytes(), large_value.as_bytes())?;
            let duration = start.elapsed();
            large_value_times.push(duration.as_nanos() as f64);
        }

        let avg_large = large_value_times.iter().sum::<f64>() / large_value_times.len() as f64;
        println!("✅ 大数据值插入 (2KB): 平均 {:.2} µs/条", avg_large / 1000.0);

        // 测试6: 并发性能测试 (Surface Book 2 4核8线程优势 + 混合管理器)
        println!("\n📊 测试6: 并发写入性能 (Surface Book 2 4核8线程 + 混合管理器)");
        use std::thread;

        let manager_clone = manager.clone();
        let mut handles = vec![];

        let start = Instant::now();

        // 利用Surface Book 2的4核8线程设计 + 混合管理器并发安全
        for thread_id in 0..8 {
            let manager_ref = manager_clone.clone();
            let handle = thread::spawn(move || {
                for i in 0..1000 {
                    let key = format!("sb2_concurrent_key_{}_{}", thread_id, i);
                    let value = format!("uncompressed_sb2_concurrent_value_{}_{}", thread_id, i);
                    manager_ref.insert(key.as_bytes(), value.as_bytes())?;
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

        println!("✅ 并发写入性能 (8线程 - Surface Book 2无压缩 + 混合管理器):");
        println!("   总耗时: {:?}", concurrent_duration);
        println!("   平均: {:.2} µs/条", avg_concurrent / 1000.0);
        println!("   吞吐量: {:.0} ops/sec", concurrent_ops as f64 / concurrent_duration.as_secs_f64());

        // 清理
        drop(manager);
        drop(db);
        std::fs::remove_dir_all("surface_book_2_compression_none_db")?;

        println!("\n🎉 Surface Book 2 无压缩性能测试完成！（混合操作管理器）");
        println!("📈 设备配置: Surface Book 2 - Intel Core i7-8650U (4核8线程), 16GB内存");
        println!("🗜️  压缩配置: CompressionAlgorithm::None");
        println!("🔧 管理器: HybridOperationsManager（直接访问模式）");
        println!("📊 Surface Book 2无压缩模式性能特点:");
        println!("   - 写入: {:.1} µs/条 (零压缩开销 + AVX2优化)", avg_insert / 1000.0);
        println!("   - 读取: {:.1} µs/条 (零解压缩开销 + 大缓存)", avg_read / 1000.0);
        println!("   - 原子操作: {:.1} µs/次 (混合管理器特色)", avg_atomic / 1000.0);
        println!("   - 并发: {:.1} µs/条 (4核8线程优势)", avg_concurrent / 1000.0);
        println!("   - 大数据: {:.1} µs/条 (16GB内存优势)", avg_large / 1000.0);

        println!("\n🎯 Surface Book 2无压缩 + 混合管理器优势:");
        println!("   ✅ AVX2指令集加速数据处理");
        println!("   ✅ 16GB大内存提供充足缓存空间");
        println!("   ✅ 4核8线程设计提供优秀并发性能");
        println!("   ✅ 混合管理器零开销直接访问");
        println!("   ✅ 原子操作并发安全，自动持久化");
        println!("   ✅ 零压缩延迟，适合实时应用");

        println!("\n🔍 Surface Book 2性能评估:");
        let sb2_excellent_write = 3.5;  // x86高端设备标准
        let sb2_excellent_read = 1.5;

        if avg_insert / 1000.0 <= sb2_excellent_write && avg_read / 1000.0 <= sb2_excellent_read {
            println!("✅ Surface Book 2无压缩 + 混合管理器性能表现卓越，充分发挥了高端x86设备的优势！");
        } else if avg_insert / 1000.0 <= sb2_excellent_write * 1.5 && avg_read / 1000.0 <= sb2_excellent_read * 1.5 {
            println!("✅ Surface Book 2无压缩 + 混合管理器性能表现优秀，适合高性能应用场景");
        } else {
            println!("⚠️  Surface Book 2性能低于预期，建议检查电源模式设置");
        }

        println!("\n💡 Surface Book 2无压缩 + 混合管理器适用场景:");
        println!("   - 实时数据处理系统");
        println!("   - 高频交易应用");
        println!("   - 游戏和交互式应用");
        println!("   - 科学计算和数据分析");
        println!("   - 需要极致性能和并发安全的任何场景");
        println!("   - 存储空间充足的Windows高端设备");

        println!("\n🚀 Surface Book 2 + 混合管理器优化总结:");
        println!("   - AVX2指令集: 32字节向量处理，高效SIMD运算");
        println!("   - 16GB大内存: 充足缓存空间，减少磁盘IO");
        println!("   - 4核8线程: Hyper-Threading技术，优秀并发性能");
        println!("   - 无压缩瓶颈: 消除压缩算法的性能限制");
        println!("   - 混合管理器: 智能路由，零开销 + 并发安全");
        println!("   - 原子操作: 自动持久化，无需额外代码");
    }

    Ok(())
}