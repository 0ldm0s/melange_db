use melange_db::{Db, Config, platform_utils, atomic_worker::AtomicWorker};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 原子操作Worker测试");
    println!("==================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("atomic_worker_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建AtomicWorker");
    println!("------------------------");

    // 创建AtomicWorker
    let atomic_worker = AtomicWorker::new(db.clone());
    let atomic_worker = Arc::new(atomic_worker);
    println!("✅ AtomicWorker创建成功");

    println!("\n📋 测试2: 基本原子递增");
    println!("--------------------");

    // 测试基本递增功能
    let val1 = atomic_worker.increment("test_counter".to_string(), 1)?;
    println!("  第1次递增: {}", val1);

    let val2 = atomic_worker.increment("test_counter".to_string(), 1)?;
    println!("  第2次递增: {}", val2);

    let val3 = atomic_worker.increment("test_counter".to_string(), 5)?;
    println!("  步长5递增: {}", val3);

    let current = atomic_worker.get("test_counter".to_string())?;
    println!("  当前计数器值: {:?}", current);

    if current == Some(7) {
        println!("  ✅ 基本递增测试通过");
    } else {
        println!("  ❌ 基本递增测试失败: 预期7，实际{:?}", current);
    }

    println!("\n📋 测试3: 简单2线程并发测试");
    println!("-------------------------");

    let mut handles = vec![];

    // 启动2个线程，每个线程递增10次，步长为2
    for thread_id in 0..2 {
        let atomic_worker_clone: Arc<AtomicWorker> = Arc::clone(&atomic_worker);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                match atomic_worker_clone.increment("concurrent_counter".to_string(), 2) {
                    Ok(value) => {
                        if i % 5 == 0 {
                            println!("  线程{} 第{}次递增: {}", thread_id, i + 1, value);
                        }
                    }
                    Err(e) => {
                        eprintln!("  线程{} 递增失败: {:?}", thread_id, e);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let final_count = atomic_worker.get("concurrent_counter".to_string())?;
    let expected = 2 * 10 * 2; // 2个线程 * 10次 * 步长2 = 40
    println!("  最终计数器值: {:?}", final_count);
    println!("  预期值: {}", expected);

    match final_count {
        Some(count) if count == expected => {
            println!("  ✅ 2线程并发测试通过");
        }
        Some(count) => {
            println!("  ❌ 测试失败: 实际值{} != 预期值{}", count, expected);
        }
        None => {
            println!("  ❌ 测试失败: 计数器不存在");
        }
    }

    println!("\n📋 测试4: 重置计数器");
    println!("------------------");

    atomic_worker.reset("test_counter".to_string(), 100)?;
    let reset_value = atomic_worker.get("test_counter".to_string())?;
    println!("  重置后的值: {:?}", reset_value);

    if reset_value == Some(100) {
        println!("  ✅ 重置计数器测试通过");
    } else {
        println!("  ❌ 重置计数器测试失败: 预期100，实际{:?}", reset_value);
    }

    println!("\n📋 测试5: 持久化验证");
    println!("------------------");

    // 创建新的AtomicWorker实例来测试持久化
    let atomic_worker2 = AtomicWorker::new(db.clone());

    // 预热计数器
    let loaded_count = atomic_worker2.preload_counters(&db)?;
    println!("  预热加载了 {} 个计数器", loaded_count);

    let persisted_value = atomic_worker2.get("test_counter".to_string())?;
    println!("  持久化后的test_counter值: {:?}", persisted_value);

    if persisted_value == Some(100) {
        println!("  ✅ 持久化验证通过");
    } else {
        println!("  ❌ 持久化验证失败: 预期100，实际{:?}", persisted_value);
    }

    println!("\n🎉 所有AtomicWorker测试完成！");

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}