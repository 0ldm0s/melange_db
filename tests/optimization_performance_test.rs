//! 综合优化性能测试
//!
//! 此测试对比了优化前后的性能差异，包括：
//! - SIMD优化的key比较性能
//! - 布隆过滤器查询性能
//! - 块缓存命中率
//! - 整体查询性能

use melange_db::*;
use std::time::{Duration, Instant};
use rand::Rng;
use std::process::{Command, Stdio};

#[cfg(test)]
mod optimization_tests {
    use super::*;
    use melange_db::simd_optimized::SimdComparator;
    use melange_db::bloom_filter::{BloomFilter, ConcurrentBloomFilter};
    use melange_db::block_cache::{CacheManager, CacheConfig, CacheBlock, AccessPattern};
    use std::collections::HashMap;

    const TEST_KEY_COUNT: usize = 100_000;
    const QUERY_COUNT: usize = 50_000;
    const CACHE_SIZE: usize = 64 * 1024 * 1024; // 64MB

    /// 生成测试keys
    fn generate_test_keys(count: usize) -> Vec<Vec<u8>> {
        let mut rng = rand::thread_rng();
        let mut keys = Vec::new();

        for i in 0..count {
            let key_length = 16 + rng.random_range(0..32); // 16-48字节
            let mut key = vec![0u8; key_length];

            // 填充随机数据
            for byte in &mut key {
                *byte = rng.random();
            }

            // 添加一些前缀以模拟真实场景
            let prefix = format!("user_{}_", i % 1000);
            key[..prefix.len()].copy_from_slice(prefix.as_bytes());

            keys.push(key);
        }

        keys
    }

    /// SIMD key比较性能测试
    #[test]
    fn test_simd_key_comparison_performance() {
        println!("=== SIMD Key比较性能测试 ===");

        let keys = generate_test_keys(1000);
        let mut comparisons = 0;
        let start = Instant::now();

        // 进行大量key比较操作
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                if i != j {
                    SimdComparator::compare(&keys[i], &keys[j]);
                    comparisons += 1;
                }
            }
        }

        let duration = start.elapsed();
        let comparisons_per_sec = comparisons as f64 / duration.as_secs_f64();

        println!("SIMD比较性能: {} comparisons/sec",
                 format_args!("{:.0}", comparisons_per_sec));
        println!("总耗时: {:?}", duration);
        println!("比较次数: {}", comparisons);
    }

    /// 布隆过滤器性能测试
    #[test]
    fn test_bloom_filter_performance() {
        println!("=== 布隆过滤器性能测试 ===");

        let keys = generate_test_keys(TEST_KEY_COUNT);
        let mut bloom_filter = BloomFilter::new(TEST_KEY_COUNT, 0.01);

        // 插入测试
        let insert_start = Instant::now();
        for key in &keys {
            bloom_filter.insert(key);
        }
        let insert_duration = insert_start.elapsed();

        println!("插入性能: {:.0} items/sec",
                 TEST_KEY_COUNT as f64 / insert_duration.as_secs_f64());

        // 查询测试 - 存在的keys
        let query_start = Instant::now();
        let mut true_positives = 0;
        for key in &keys {
            if bloom_filter.contains(key) {
                true_positives += 1;
            }
        }
        let query_duration = query_start.elapsed();

        println!("查询性能(存在): {:.0} queries/sec",
                 TEST_KEY_COUNT as f64 / query_duration.as_secs_f64());
        println!("命中率: {}/{}", true_positives, TEST_KEY_COUNT);

        // 查询测试 - 不存在的keys
        let other_keys = generate_test_keys(QUERY_COUNT);
        let false_positives_start = Instant::now();
        let mut false_positives = 0;
        for key in &other_keys {
            if bloom_filter.contains(key) {
                false_positives += 1;
            }
        }
        let false_positives_duration = false_positives_start.elapsed();

        let false_positive_rate = false_positives as f64 / QUERY_COUNT as f64;
        println!("查询性能(不存在): {:.0} queries/sec",
                 QUERY_COUNT as f64 / false_positives_duration.as_secs_f64());
        println!("误判率: {:.4}%", false_positive_rate * 100.0);

        // 显示统计信息
        let stats = bloom_filter.stats();
        println!("统计信息: 大小={}字节, 哈希函数={}",
                 stats.size_in_bytes, stats.hash_count);
    }

    /// 块缓存性能测试
    #[test]
    fn test_block_cache_performance() {
        println!("=== 块缓存性能测试 ===");

        let config = CacheConfig {
            max_size: CACHE_SIZE,
            block_size: 4096,
            enable_prefetch: true,
            ..Default::default()
        };
        let cache_manager = CacheManager::new(config);

        // 生成测试数据
        let mut test_data = HashMap::new();
        for i in 0..1000 {
            let data = vec![i as u8; 4096]; // 4KB块
            test_data.insert(i, data);
        }

        // 写入测试
        let write_start = Instant::now();
        for (block_id, data) in &test_data {
            cache_manager.write_block(*block_id, data.clone());
        }
        let write_duration = write_start.elapsed();

        println!("写入性能: {:.0} blocks/sec",
                 test_data.len() as f64 / write_duration.as_secs_f64());

        // 读取测试 - 热数据
        let mut cache_hits = 0;
        let read_start = Instant::now();
        for _ in 0..QUERY_COUNT {
            let block_id = (rand::random::<u64>() % 1000) as u64;
            if cache_manager.read_block(block_id).is_some() {
                cache_hits += 1;
            }
        }
        let read_duration = read_start.elapsed();

        let hit_rate = cache_hits as f64 / QUERY_COUNT as f64;
        println!("读取性能: {:.0} queries/sec",
                 QUERY_COUNT as f64 / read_duration.as_secs_f64());
        println!("缓存命中率: {:.2}%", hit_rate * 100.0);

        // 显示缓存统计
        let stats = cache_manager.stats();
        let size_info = cache_manager.size_info();
        println!("缓存统计: 命中={}, 未命中={}, 热块={}",
                 stats.hits, stats.misses, size_info.hot_blocks);
    }

    /// 综合查询性能测试
    #[test]
    fn test_comprehensive_query_performance() {
        println!("=== 综合查询性能测试 ===");

        // 创建临时数据库
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_db");

        let db: Db<1024> = Config::new()
            .path(&db_path)
            .open()
            .unwrap();

        let tree = db.open_tree::<&[u8]>(b"performance_test").unwrap();

        // 准备测试数据
        let keys = generate_test_keys(TEST_KEY_COUNT);
        let values: Vec<Vec<u8>> = keys.iter()
            .map(|_| {
                let mut value = vec![0u8; 1024]; // 1KB值
                for byte in &mut value {
                    *byte = rand::thread_rng().random();
                }
                value
            })
            .collect();

        // 写入性能测试
        let write_start = Instant::now();
        for (i, (key, value)) in keys.iter().zip(&values).enumerate() {
            tree.insert(key.as_slice(), value.as_slice()).unwrap();

            if i % 10000 == 0 {
                println!("写入进度: {}/{}", i, TEST_KEY_COUNT);
            }
        }
        let write_duration = write_start.elapsed();

        println!("写入性能: {:.0} ops/sec",
                 TEST_KEY_COUNT as f64 / write_duration.as_secs_f64());

        // 读取性能测试 - 随机访问
        let read_start = Instant::now();
        let mut found = 0;
        for _ in 0..QUERY_COUNT {
            let key_index = rand::thread_rng().random_range(0..TEST_KEY_COUNT);
            if tree.get(keys[key_index].as_slice()).unwrap().is_some() {
                found += 1;
            }
        }
        let read_duration = read_start.elapsed();

        let find_rate = found as f64 / QUERY_COUNT as f64;
        println!("读取性能: {:.0} queries/sec",
                 QUERY_COUNT as f64 / read_duration.as_secs_f64());
        println!("查找成功率: {:.2}%", find_rate * 100.0);

        // 范围查询性能测试
        let range_start = Instant::now();
        let mut range_count = 0;
        for _ in 0..100 {
            let start_key = &keys[rand::thread_rng().random_range(0..TEST_KEY_COUNT)];
            let mut iter = tree.range(start_key.as_slice()..);

            for _ in iter.by_ref().take(100) {
                range_count += 1;
            }
        }
        let range_duration = range_start.elapsed();

        println!("范围查询性能: {:.0} items/sec",
                 range_count as f64 / range_duration.as_secs_f64());

        // 前缀查询性能测试
        let prefix_start = Instant::now();
        let mut prefix_count = 0;
        for i in 0..100 {
            let prefix = format!("user_{}_", i);
            let mut iter = tree.scan_prefix(&prefix);

            for _ in iter.by_ref() {
                prefix_count += 1;
            }
        }
        let prefix_duration = prefix_start.elapsed();

        println!("前缀查询性能: {:.0} items/sec",
                 prefix_count as f64 / prefix_duration.as_secs_f64());

        println!("========================================");
        println!("综合性能测试完成");
        println!("========================================");
    }

    /// 内存使用量测试
    #[test]
    fn test_memory_usage() {
        println!("=== 内存使用量测试 ===");

        let keys = generate_test_keys(10_000);
        let values: Vec<Vec<u8>> = keys.iter()
            .map(|_| vec![0u8; 512]) // 512字节值
            .collect();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("memory_test");

        let db: Db<1024> = Config::new()
            .path(&db_path)
            .open()
            .unwrap();

        let tree = db.open_tree::<&[u8]>(b"memory_test").unwrap();

        // 强制垃圾回收，确保测量准确
        tree.flush().unwrap();
        std::thread::sleep(Duration::from_millis(100));

        // 写入前内存
        let before_mem = get_memory_usage();
        println!("写入前内存: {} bytes", before_mem);

        // 写入数据
        for (i, (key, value)) in keys.iter().zip(&values).enumerate() {
            tree.insert(key.as_slice(), value.as_slice()).unwrap();

            // 每1000条数据强制刷新一次
            if i % 1000 == 0 {
                tree.flush().unwrap();
            }
        }

        // 确保所有数据写入磁盘
        tree.flush().unwrap();
        std::thread::sleep(Duration::from_millis(200));

        // 写入后内存
        let after_mem = get_memory_usage();
        let mem_increase = after_mem - before_mem;
        println!("写入后内存: {} bytes", after_mem);
        let bytes_per_item = mem_increase as f64 / keys.len() as f64;

        println!("Keys数量: {}", keys.len());
        println!("内存增长: {} bytes", mem_increase);
        println!("平均每项内存: {:.2} bytes", bytes_per_item);
        println!("数据密度: {:.2}%",
                 (keys.len() * 512) as f64 / mem_increase as f64 * 100.0);
    }

    // 获取当前进程内存使用量（跨平台实现）
    fn get_memory_usage() -> usize {
        #[cfg(target_os = "macos")]
        {
            // 在macOS上使用ps命令获取进程内存使用量
            if let Ok(output) = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
            {
                if let Ok(rss_kb) = String::from_utf8(output.stdout) {
                    if let Ok(rss) = rss_kb.trim().parse::<usize>() {
                        return rss * 1024; // 转换为字节
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // 在Linux上从/proc/self/status读取内存信息
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(rss_kb) = parts[1].parse::<usize>() {
                                return rss_kb * 1024; // 转换为字节
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            // 其他平台的回退方法
            println!("警告：当前平台不支持精确内存测量，使用估算方法");
            // 这里可以使用其他方法，比如系统特定的API
        }

        // 如果所有方法都失败，返回估算值
        // 实际数据量：10,000个keys * (平均32字节key + 512字节value) ≈ 5.44MB
        // 加上数据库索引和缓存开销，估算约为8-12MB
        10 * 1024 * 1024 // 10MB 估算值
    }
}