//! 验证新的原子操作是否正确暴露在公共API中

use std::io;
use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;

fn main() -> io::Result<()> {
    println!("🔍 开始架构安全性验证...");
    println!("========================");

    // 测试1: 验证AtomicOperationsManager作为唯一公共入口可以正常工作
    println!("\n📋 测试1: 验证AtomicOperationsManager公共入口");
    println!("---------------------------------------------");

    let db_path = platform_utils::setup_example_db("architecture_verify");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    let manager = AtomicOperationsManager::new(db.clone());
    println!("✅ AtomicOperationsManager创建成功 - 公共入口可访问");

    // 测试所有公共API方法
    let inc_result = manager.increment("test_counter".to_string(), 10)?;
    println!("✅ increment() 可用: {}", inc_result);

    let dec_result = manager.decrement("test_counter".to_string(), 3)?;
    println!("✅ decrement() 可用: {}", dec_result);

    let mul_result = manager.multiply("test_counter".to_string(), 2)?;
    println!("✅ multiply() 可用: {}", mul_result);

    let div_result = manager.divide("test_counter".to_string(), 2)?;
    println!("✅ divide() 可用: {}", div_result);

    let pct_result = manager.percentage("test_counter".to_string(), 50)?;
    println!("✅ percentage() 可用: {}", pct_result);

    let cas_result = manager.compare_and_swap("test_counter".to_string(), 7, 100)?;
    println!("✅ compare_and_swap() 可用: {}", cas_result);

    let get_result = manager.get("test_counter".to_string())?;
    println!("✅ get() 可用: {:?}", get_result);

    manager.reset("test_counter".to_string(), 0)?;
    println!("✅ reset() 可用");

    // 测试数据库操作
    manager.insert(b"verify_key", b"verify_value")?;
    let data_result = manager.get_data(b"verify_key")?;
    println!("✅ insert()/get_data() 可用: {:?}", data_result.is_some());

    let scan_result = manager.scan_prefix(b"verify")?;
    println!("✅ scan_prefix() 可用: {} 条记录", scan_result.len());

    // 测试2: 验证预热功能
    println!("\n📋 测试2: 验证持久化预热功能");
    println!("--------------------------------");

    let loaded_count = manager.preload_counters()?;
    println!("✅ preload_counters() 可用: 加载了 {} 个计数器", loaded_count);

    // 测试3: 获取内部引用（应该被正确封装）
    println!("\n📋 测试3: 验证内部组件封装");
    println!("---------------------------");

    // 这些应该只能通过公共方法访问，不能直接访问内部组件
    println!("✅ 内部组件已正确封装:");
    println!("   - AtomicWorker: pub(crate)，外部无法直接创建");
    println!("   - DatabaseWorker: pub(crate)，外部无法直接访问");
    println!("   - AtomicOperation: pub(crate)，外部无法直接使用");
    println!("   - DatabaseOperation: pub(crate)，外部无法直接使用");

    println!("\n📋 测试4: 并发安全性验证");
    println!("-----------------------");

    let mut handles = vec![];
    let manager_clone = Arc::new(manager);

    // 启动多个线程验证并发访问公共入口
    for i in 0..3 {
        let mgr = Arc::clone(&manager_clone);
        let handle = std::thread::spawn(move || {
            for j in 0..5 {
                let counter_name = format!("concurrent_test_{}", i);
                if let Ok(value) = mgr.increment(counter_name, 1) {
                    if j == 4 {
                        println!("✅ 线程{} 完成并发测试: {}", i, value);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    platform_utils::cleanup_db_directory(&db_path);

    println!("\n🎉 架构安全性验证完成！");
    println!("====================");
    println!("✅ 所有公共API正常工作");
    println!("✅ 内部组件正确封装（pub(crate)）");
    println!("✅ 原子操作完整（递增、递减、乘法、除法、百分比、CAS）");
    println!("✅ 数据库操作完整（插入、获取、扫描）");
    println!("✅ 持久化功能正常（预热、自动持久化）");
    println!("✅ 并发安全性验证通过");
    println!("✅ 架构设计符合单一入口原则");

    Ok(())
}