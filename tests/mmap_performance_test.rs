//! MMAP性能对比测试
//!
//! 此测试用于验证MMAP是否为读取性能瓶颈

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};
use tempfile::tempfile;
use melange_db::*;
use rand::Rng;

#[cfg(unix)]
use std::os::unix::fs::FileExt;

const TEST_DATA_SIZE: usize = 10 * 1024 * 1024; // 10MB测试数据
const READ_COUNT: usize = 1000;
const BLOCK_SIZE: usize = 4096;

/// 生成测试数据
fn generate_test_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(TEST_DATA_SIZE);
    let mut rng = rand::thread_rng();
    for _ in 0..TEST_DATA_SIZE {
        data.push(rng.random::<u8>());
    }
    data
}

/// 传统read_exact_at性能测试
fn test_traditional_io(file: &File, block_size: usize) -> io::Result<(Duration, u64)> {
    let mut buffer = vec![0u8; block_size];
    let mut total_bytes_read = 0u64;
    let start = Instant::now();

    for i in 0..READ_COUNT {
        let offset = (i * block_size % TEST_DATA_SIZE) as u64;
        file.read_exact_at(&mut buffer, offset)?;
        total_bytes_read += block_size as u64;
    }

    let duration = start.elapsed();
    Ok((duration, total_bytes_read))
}

/// MMAP性能测试
#[cfg(unix)]
fn test_mmap_io(file: &File, block_size: usize) -> io::Result<(Duration, u64)> {
    use std::os::unix::io::AsRawFd;
    use libc::{mmap, munmap, PROT_READ, MAP_PRIVATE};

    let fd = file.as_raw_fd();
    let file_len = file.metadata()?.len() as usize;

    let start = Instant::now();
    let mut total_bytes_read = 0u64;

    unsafe {
        // 映射整个文件
        let mmap_ptr = mmap(
            std::ptr::null_mut(),
            file_len,
            PROT_READ,
            MAP_PRIVATE,
            fd,
            0,
        );

        if mmap_ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        let mmap_ptr = mmap_ptr as *mut u8;

        // 执行读取测试
        for i in 0..READ_COUNT {
            let offset = i * block_size % TEST_DATA_SIZE;
            let end_offset = (offset + block_size).min(file_len);
            let buffer = std::slice::from_raw_parts(mmap_ptr.add(offset), end_offset - offset);

            // 模拟数据处理 - 计算校验和
            let _checksum: u32 = buffer.iter().map(|&b| b as u32).sum();
            total_bytes_read += buffer.len() as u64;
        }

        munmap(mmap_ptr as *mut libc::c_void, file_len);
    }

    let duration = start.elapsed();
    Ok((duration, total_bytes_read))
}

/// 缓冲池IO性能测试
fn test_buffered_io(file: &mut File, block_size: usize) -> io::Result<(Duration, u64)> {
    let mut buffer = vec![0u8; block_size];
    let mut total_bytes_read = 0u64;
    let start = Instant::now();

    for i in 0..READ_COUNT {
        let offset = (i * block_size % TEST_DATA_SIZE) as u64;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buffer)?;
        total_bytes_read += block_size as u64;
    }

    let duration = start.elapsed();
    Ok((duration, total_bytes_read))
}

#[cfg(test)]
mod mmap_performance_tests {
    use super::*;

    #[test]
    fn test_mmap_vs_traditional_io() {
        // 创建测试文件
        let test_data = generate_test_data();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(&test_data).unwrap();
        temp_file.sync_all().unwrap();

        println!("MMAP性能对比测试");
        println!("测试数据大小: {} MB", TEST_DATA_SIZE / (1024 * 1024));
        println!("读取次数: {}", READ_COUNT);
        println!("块大小: {} bytes", BLOCK_SIZE);
        println!("========================================");

        // 测试传统IO
        let mut file = File::open(&file_path).unwrap();
        let (traditional_duration, traditional_bytes) = test_traditional_io(&file, BLOCK_SIZE).unwrap();
        let traditional_mb_per_sec = (traditional_bytes as f64 / (1024.0 * 1024.0))
            / traditional_duration.as_secs_f64();

        println!("传统 read_exact_at:");
        println!("  总耗时: {:?}", traditional_duration);
        println!("  吞吐量: {:.2} MB/s", traditional_mb_per_sec);

        // 测试缓冲IO
        let mut file = File::open(&file_path).unwrap();
        let (buffered_duration, buffered_bytes) = test_buffered_io(&mut file, BLOCK_SIZE).unwrap();
        let buffered_mb_per_sec = (buffered_bytes as f64 / (1024.0 * 1024.0))
            / buffered_duration.as_secs_f64();

        println!("缓冲 IO:");
        println!("  总耗时: {:?}", buffered_duration);
        println!("  吞吐量: {:.2} MB/s", buffered_mb_per_sec);

        // 测试MMAP IO (仅在Unix系统上)
        #[cfg(unix)]
        {
            let file = File::open(&file_path).unwrap();
            let (mmap_duration, mmap_bytes) = test_mmap_io(&file, BLOCK_SIZE).unwrap();
            let mmap_mb_per_sec = (mmap_bytes as f64 / (1024.0 * 1024.0))
                / mmap_duration.as_secs_f64();

            println!("MMAP IO:");
            println!("  总耗时: {:?}", mmap_duration);
            println!("  吞吐量: {:.2} MB/s", mmap_mb_per_sec);

            // 计算性能提升
            let speedup_vs_traditional = mmap_mb_per_sec / traditional_mb_per_sec;
            let speedup_vs_buffered = mmap_mb_per_sec / buffered_mb_per_sec;

            println!("========================================");
            println!("性能对比:");
            println!("  MMAP vs 传统: {:.2}x {}", speedup_vs_traditional,
                if speedup_vs_traditional > 1.0 { "更快" } else { "更慢" });
            println!("  MMAP vs 缓冲: {:.2}x {}", speedup_vs_buffered,
                if speedup_vs_buffered > 1.0 { "更快" } else { "更慢" });
        }

        #[cfg(not(unix))]
        {
            println!("MMAP测试仅在Unix系统上运行");
        }
    }

    #[test]
    fn test_random_access_patterns() {
        // 创建测试文件
        let test_data = generate_test_data();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(&test_data).unwrap();
        temp_file.sync_all().unwrap();

        println!("随机访问模式测试");
        println!("========================================");

        let mut rng = rand::thread_rng();
        let file = File::open(&file_path).unwrap();

        let start = Instant::now();
        let mut total_bytes = 0;

        // 模拟数据库的随机访问模式
        for _ in 0..READ_COUNT {
            // 随机选择块大小（模拟不同大小的key-value）
            let block_size = 512 + rng.random_range(0..4096);
            let offset = rng.random_range(0..(TEST_DATA_SIZE - block_size)) as u64;

            let mut buffer = vec![0u8; block_size];
            if let Ok(_) = file.read_exact_at(&mut buffer, offset) {
                total_bytes += block_size;
            }
        }

        let duration = start.elapsed();
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

        println!("随机访问性能:");
        println!("  总读取量: {} bytes", total_bytes);
        println!("  总耗时: {:?}", duration);
        println!("  吞吐量: {:.2} MB/s", mb_per_sec);
    }
}

/// 分析当前melange_db的IO模式
pub fn analyze_current_io_pattern() {
    println!("当前melange_db IO模式分析");
    println!("========================================");
    println!("当前实现:");
    println!("  - 使用 read_exact_at/write_all_at 系统调用");
    println!("  - 每次读取都进行文件系统调用");
    println!("  - 包含CRC32校验，增加CPU开销");
    println!("  - 可能的优化点:");
    println!("    1. 实现预读机制减少系统调用");
    println!("    2. 考虑小文件的MMAP映射");
    println!("    3. 实现自适应IO策略");
    println!("    4. 减少不必要的内存拷贝");
}

/// 建议的IO优化策略
pub fn suggest_io_optimization_strategies() {
    println!("IO优化建议");
    println!("========================================");
    println!("1. 预读优化:");
    println!("   - 预测访问模式，提前读取相邻数据");
    println!("   - 减少系统调用次数");
    println!("");
    println!("2. 混合IO策略:");
    println!("   - 小文件使用MMAP，大文件使用传统IO");
    println!("   - 热点数据使用MMAP，冷数据使用传统IO");
    println!("");
    println!("3. 零拷贝优化:");
    println!("   - 避免不必要的数据复制");
    println!("   - 直接操作映射内存");
    println!("");
    println!("4. 异步IO:");
    println!("   - 使用io_uring(Linux)或kqueue(BSD)");
    println!("   - 并行处理多个IO请求");
}