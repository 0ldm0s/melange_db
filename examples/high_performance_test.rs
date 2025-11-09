//! 高压力性能测试
//!
//! 测试优化后的统一入口在高负载下的性能表现

use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 高压力性能测试");
    println!("==================");
    println!("测试优化后的统一入口性能");

    // 创建数据库配置
    let config = Config::new()
        .path("high_perf_test_db")
        .cache_capacity_bytes(64 * 1024 * 1024); // 64MB缓存

    // 打开数据库
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 创建统一路由器
    let manager = Arc::new(AtomicOperationsManager::new(db.clone()));

    println!("✅ 数据库和统一路由器初始化完成");
    println!();

    // 测试1: 纯原子操作性能
    println!("📊 测试1: 纯原子操作性能");
    test_atomic_operations(&manager, Duration::from_secs(30))?;

    println!();

    // 测试2: 混合操作性能
    println!("📊 测试2: 混合操作性能");
    test_mixed_operations(&manager, Duration::from_secs(30))?;

    println!();

    // 测试3: 高并发压力测试
    println!("📊 测试3: 高并发压力测试");
    test_concurrent_stress(&manager, Duration::from_secs(30))?;

    println!();
    println!("🎉 所有性能测试完成！");

    // 清理测试数据库
    std::fs::remove_dir_all("high_perf_test_db").ok();

    Ok(())
}

fn test_atomic_operations(manager: &Arc<AtomicOperationsManager>, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
    println!("运行{}秒纯原子操作测试...", duration.as_secs());

    let start_time = Instant::now();
    let mut operation_count = 0;

    while start_time.elapsed() < duration {
        // 执行各种原子操作
        manager.increment("perf_counter".to_string(), 1)?;
        operation_count += 1;

        if operation_count % 10 == 0 {
            manager.decrement("perf_counter".to_string(), 1)?;
            operation_count += 1;
        }

        if operation_count % 20 == 0 {
            manager.multiply("perf_counter".to_string(), 2)?;
            operation_count += 1;
        }

        if operation_count % 30 == 0 {
            let _ = manager.get("perf_counter".to_string())?;
            operation_count += 1;
        }
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = operation_count as f64 / elapsed.as_secs_f64();

    println!("✅ 纯原子操作: {} 次, {:.1} ops/sec", operation_count, ops_per_sec);
    Ok(())
}

fn test_mixed_operations(manager: &Arc<AtomicOperationsManager>, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
    println!("运行{}秒混合操作测试...", duration.as_secs());

    let start_time = Instant::now();
    let mut operation_count = 0;
    let mut data_counter = 0;

    while start_time.elapsed() < duration {
        // 原子操作
        if operation_count % 3 == 0 {
            manager.increment("mixed_counter".to_string(), 1)?;
        } else {
            // 数据库操作
            let key = format!("mixed_key_{}", data_counter);
            let value = format!("mixed_value_{}", data_counter);
            manager.insert(key.as_bytes(), value.as_bytes())?;

            // 偶尔读取
            if data_counter % 10 == 0 {
                let _ = manager.get_data(key.as_bytes())?;
            }

            // 偶尔删除
            if data_counter % 20 == 0 && data_counter > 10 {
                let delete_key = format!("mixed_key_{}", data_counter - 10);
                let _ = manager.remove(delete_key.as_bytes())?;
            }

            data_counter += 1;
        }

        operation_count += 1;

        // 每100次操作短暂休眠，模拟真实场景
        if operation_count % 100 == 0 {
            std::thread::sleep(Duration::from_micros(500));
        }
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = operation_count as f64 / elapsed.as_secs_f64();

    println!("✅ 混合操作: {} 次, {:.1} ops/sec (数据操作: {})", operation_count, ops_per_sec, data_counter);
    Ok(())
}

fn test_concurrent_stress(manager: &Arc<AtomicOperationsManager>, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
    println!("运行{}秒高并发测试...", duration.as_secs());

    let manager_clone = manager.clone();
    let start_time = Instant::now();
    let mut handles = Vec::new();

    // 创建多个线程进行并发操作
    for thread_id in 0..4 {
        let manager_ref = manager_clone.clone();
        let handle = std::thread::spawn(move || -> std::io::Result<usize> {
            let mut thread_ops = 0;

            while start_time.elapsed() < duration {
                // 不同线程执行不同类型的操作
                match thread_id {
                    0 => {
                        // 线程0: 主要做原子操作
                        manager_ref.increment(format!("thread_{}_counter", thread_id), 1)?;
                        if thread_ops % 5 == 0 {
                            manager_ref.get(format!("thread_{}_counter", thread_id))?;
                        }
                    }
                    1 => {
                        // 线程1: 主要做数据库插入
                        let key = format!("thread_{}_key_{}", thread_id, thread_ops);
                        let value = format!("thread_{}_value_{}", thread_id, thread_ops);
                        manager_ref.insert(key.as_bytes(), value.as_bytes())?;
                    }
                    2 => {
                        // 线程2: 主要做数据库读取
                        if thread_ops > 10 {
                            let read_key = format!("thread_1_key_{}", thread_ops - 10);
                            let _ = manager_ref.get_data(read_key.as_bytes())?;
                        }
                    }
                    3 => {
                        // 线程3: 混合操作
                        if thread_ops % 2 == 0 {
                            manager_ref.decrement(format!("thread_{}_counter", thread_id), 1)?;
                        } else {
                            let scan_key = format!("thread_1_key");
                            let _ = manager_ref.scan_prefix(scan_key.as_bytes())?;
                        }
                    }
                    _ => {}
                }

                thread_ops += 1;

                // 每50次操作短暂休眠
                if thread_ops % 50 == 0 {
                    std::thread::sleep(Duration::from_micros(200));
                }
            }

            Ok(thread_ops)
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    let mut total_operations = 0;
    for handle in handles {
        total_operations += handle.join().unwrap()?;
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = total_operations as f64 / elapsed.as_secs_f64();

    println!("✅ 高并发测试: {} 次, {:.1} ops/sec (4线程)", total_operations, ops_per_sec);
    Ok(())
}