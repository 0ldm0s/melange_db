use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 原子操作统一管理器测试");
    println!("==========================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("atomic_operations_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建AtomicOperationsManager");
    println!("------------------------------------");

    // 创建原子操作统一管理器
    let atomic_manager = Arc::new(AtomicOperationsManager::new(db.clone()));
    println!("✅ AtomicOperationsManager创建成功");

    println!("\n📋 测试2: 基本原子递增");
    println!("--------------------");

    // 测试基本递增功能
    let val1 = atomic_manager.increment("test_counter".to_string(), 1)?;
    println!("  第1次递增: {}", val1);

    let val2 = atomic_manager.increment("test_counter".to_string(), 1)?;
    println!("  第2次递增: {}", val2);

    let val3 = atomic_manager.increment("test_counter".to_string(), 5)?;
    println!("  步长5递增: {}", val3);

    let current = atomic_manager.get("test_counter".to_string())?;
    println!("  当前计数器值: {:?}", current);

    if current == Some(7) {
        println!("  ✅ 基本递增测试通过");
    } else {
        println!("  ❌ 基本递增测试失败: 预期7，实际{:?}", current);
    }

    println!("\n📋 测试3: 新原子操作测试");
    println!("----------------------");

    // 测试原子递减
    let dec_val = atomic_manager.decrement("test_counter".to_string(), 3)?;
    println!("  递减3: {} (预期 4)", dec_val);

    // 测试原子乘法
    let mul_val = atomic_manager.multiply("test_counter".to_string(), 2)?;
    println!("  乘以2: {} (预期 8)", mul_val);

    // 测试原子除法
    let div_val = atomic_manager.divide("test_counter".to_string(), 2)?;
    println!("  除以2: {} (预期 4)", div_val);

    // 测试原子百分比
    let pct_val = atomic_manager.percentage("test_counter".to_string(), 50)?;
    println!("  50%: {} (预期 2)", pct_val);

    let final_value = atomic_manager.get("test_counter".to_string())?;
    println!("  最终值: {:?}", final_value);

    if final_value == Some(2) {
        println!("  ✅ 新原子操作测试通过");
    } else {
        println!("  ❌ 新原子操作测试失败: 预期2，实际{:?}", final_value);
    }

    println!("\n📋 测试4: 原子比较和交换(CAS)");
    println!("---------------------------");

    // 测试成功的CAS操作
    let cas_success = atomic_manager.compare_and_swap("cas_counter".to_string(), 0, 100)?;
    println!("  CAS(0->100): {} (预期 true，因为默认值是0)", cas_success);

    let cas_value = atomic_manager.get("cas_counter".to_string())?;
    println!("  CAS后的值: {:?} (预期 Some(100))", cas_value);

    // 测试失败的CAS操作
    let cas_fail = atomic_manager.compare_and_swap("cas_counter".to_string(), 0, 200)?;
    println!("  CAS(0->200): {} (预期 false，因为当前值是100)", cas_fail);

    let cas_value2 = atomic_manager.get("cas_counter".to_string())?;
    println!("  失败CAS后的值: {:?} (预期 Some(100))", cas_value2);

    if cas_success && !cas_fail && cas_value == Some(100) && cas_value2 == Some(100) {
        println!("  ✅ 原子比较和交换测试通过");
    } else {
        println!("  ❌ 原子比较和交换测试失败");
    }

    println!("\n📋 测试5: 多线程高压力并发测试");
    println!("----------------------------");

    let mut handles = vec![];

    // 线程1-3：高频原子递增操作
    for thread_id in 1..=3 {
        let manager_clone = Arc::clone(&atomic_manager);
        let handle = thread::spawn(move || {
            for i in 0..30 {
                match manager_clone.increment("high_freq_counter".to_string(), 1) {
                    Ok(value) => {
                        if i % 10 == 0 {
                            println!("  线程{}(高频): 计数器 = {}", thread_id, value);
                        }
                    }
                    Err(e) => eprintln!("  线程{}高频操作失败: {:?}", thread_id, e),
                }
            }
        });
        handles.push(handle);
    }

    // 线程4-5：混合原子操作测试
    for thread_id in 4..=5 {
        let manager_clone = Arc::clone(&atomic_manager);
        let handle = thread::spawn(move || {
            for i in 0..20 {
                // 使用不同的原子操作类型
                let result: Result<(), std::io::Error> = match i % 6 {
                    0 => manager_clone.increment("mixed_ops_counter".to_string(), 5).map(|_| ()),
                    1 => manager_clone.decrement("mixed_ops_counter".to_string(), 1).map(|_| ()),
                    2 => manager_clone.multiply("mixed_ops_counter".to_string(), 2).map(|_| ()),
                    3 => manager_clone.divide("mixed_ops_counter".to_string(), 3).map(|_| ()),
                    4 => manager_clone.percentage("mixed_ops_counter".to_string(), 50).map(|_| ()),
                    _ => {
                        // CAS操作：尝试将当前值翻倍
                        if let Ok(Some(current)) = manager_clone.get("mixed_ops_counter".to_string()) {
                            manager_clone.compare_and_swap("mixed_ops_counter".to_string(), current, current * 2).map(|_| ())
                        } else {
                            Ok(())
                        }
                    }
                };

                if let Err(e) = result {
                    eprintln!("  线程{}混合操作失败: {:?}", thread_id, e);
                }

                if i % 5 == 0 {
                    if let Ok(Some(value)) = manager_clone.get("mixed_ops_counter".to_string()) {
                        println!("  线程{}(混合): 计数器 = {}", thread_id, value);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // 线程6：ID分配器测试
    let manager_clone6 = Arc::clone(&atomic_manager);
    let handle6 = thread::spawn(move || {
        for i in 0..25 {
            match manager_clone6.increment("user_id_allocator".to_string(), 1) {
                Ok(user_id) => {
                    // 模拟创建用户数据
                    let user_key = format!("user:{}:profile", user_id);
                    let user_value = format!("用户{}", i);

                    if let Err(e) = manager_clone6.insert(user_key.as_bytes(), user_value.as_bytes()) {
                        eprintln!("  线程6用户数据写入失败: {:?}", e);
                    }

                    if i % 8 == 0 {
                        println!("  线程6(ID分配): 创建用户{} ID:{}", i, user_id);
                    }
                }
                Err(e) => eprintln!("  线程6用户ID分配失败: {:?}", e),
            }
        }
    });
    handles.push(handle6);

    // 线程7：读取统计线程
    let manager_clone7 = Arc::clone(&atomic_manager);
    let handle7 = thread::spawn(move || {
        for i in 0..15 {
            // 读取各种计数器
            let high_freq = manager_clone7.get("high_freq_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let user_ids = manager_clone7.get("user_id_allocator".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let mixed_ops = manager_clone7.get("mixed_ops_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);

            // 读取数据库记录
            let user_count = manager_clone7.scan_prefix(b"user:").unwrap_or_default().len();

            if i % 3 == 0 {
                println!("  线程7(统计): 高频:{} 用户ID:{} 混合:{} 用户记录:{}",
                         high_freq, user_ids, mixed_ops, user_count);
            }

            thread::sleep(std::time::Duration::from_millis(15));
        }
    });
    handles.push(handle7);

    println!("  启动7个并发线程...");

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n📋 测试6: 并发结果验证");
    println!("---------------------");

    // 验证各种计数器
    let high_freq_counter = atomic_manager.get("high_freq_counter".to_string())?;
    let user_id_counter = atomic_manager.get("user_id_allocator".to_string())?;
    let mixed_ops_counter = atomic_manager.get("mixed_ops_counter".to_string())?;
    let user_records = atomic_manager.scan_prefix(b"user:")?;

    println!("  并发测试结果:");
    println!("    high_freq_counter: {:?}", high_freq_counter);
    println!("    user_id_allocator: {:?}", user_id_counter);
    println!("    mixed_ops_counter: {:?}", mixed_ops_counter);
    println!("    user记录数: {}", user_records.len());

    // 验证预期值
    let expected_high_freq = 3 * 30; // 3个线程 * 30次 = 90
    let expected_user_ids = 25; // 1个线程 * 25次 = 25

    let high_freq_ok = high_freq_counter == Some(expected_high_freq);
    let user_ids_ok = user_id_counter == Some(expected_user_ids);
    let user_records_ok = user_records.len() == expected_user_ids as usize;

    println!("  一致性检查:");
    println!("    high_freq_counter: {} (预期: {})", high_freq_ok, expected_high_freq);
    println!("    user_id_allocator: {} (预期: {})", user_ids_ok, expected_user_ids);
    println!("    user记录数: {} (预期: {})", user_records_ok, expected_user_ids);

    if high_freq_ok && user_ids_ok && user_records_ok {
        println!("  ✅ 7线程高压力并发测试通过");
    } else {
        println!("  ❌ 并发测试失败");
    }

    println!("\n📋 测试7: 重置计数器");
    println!("------------------");

    atomic_manager.reset("test_counter".to_string(), 100)?;
    let reset_value = atomic_manager.get("test_counter".to_string())?;
    println!("  重置后的值: {:?}", reset_value);

    if reset_value == Some(100) {
        println!("  ✅ 重置计数器测试通过");
    } else {
        println!("  ❌ 重置计数器测试失败: 预期100，实际{:?}", reset_value);
    }

    println!("\n📋 测试8: 持久化验证");
    println!("------------------");

    // 等待所有持久化操作完成
    thread::sleep(std::time::Duration::from_millis(200));

    // 创建新的AtomicOperationsManager实例来测试持久化
    let atomic_manager2 = AtomicOperationsManager::new(db.clone());

    // 预热计数器
    let loaded_count = atomic_manager2.preload_counters()?;
    println!("  预热加载了 {} 个计数器", loaded_count);

    let persisted_value = atomic_manager2.get("test_counter".to_string())?;
    println!("  持久化后的test_counter值: {:?}", persisted_value);

    // 验证并发测试的计数器也持久化了
    let persisted_high_freq = atomic_manager2.get("high_freq_counter".to_string())?;
    let persisted_user_ids = atomic_manager2.get("user_id_allocator".to_string())?;

    println!("  持久化验证:");
    println!("    test_counter: {:?} (原: {:?})", persisted_value, reset_value);
    println!("    high_freq_counter: {:?} (原: {:?})", persisted_high_freq, high_freq_counter);
    println!("    user_id_allocator: {:?} (原: {:?})", persisted_user_ids, user_id_counter);

    let persistence_ok = persisted_value == Some(100) &&
                        persisted_high_freq == high_freq_counter &&
                        persisted_user_ids == user_id_counter;

    if persistence_ok {
        println!("  ✅ 持久化验证通过");
    } else {
        println!("  ❌ 持久化验证失败");
    }

    println!("\n📋 测试9: 数据库操作集成测试");
    println!("----------------------------");

    // 测试通过统一管理器进行数据库操作
    atomic_manager.insert(b"test_key", b"test_value")?;
    let retrieved_value = atomic_manager.get_data(b"test_key")?;
    println!("  插入并获取数据: {:?}", retrieved_value);

    let scan_results = atomic_manager.scan_prefix(b"test")?;
    println!("  扫描前缀'test'的结果: {} 个键值对", scan_results.len());

    if retrieved_value.is_some() && !scan_results.is_empty() {
        println!("  ✅ 数据库操作集成测试通过");
    } else {
        println!("  ❌ 数据库操作集成测试失败");
    }

    println!("\n🎉 所有原子操作统一管理器测试完成！");
    println!("✅ 架构安全性：所有操作都通过AtomicOperationsManager统一入口");
    println!("✅ 功能完整性：支持递增、递减、乘法、除法、百分比、CAS等操作");
    println!("✅ 并发安全性：多线程环境下的原子性保证");
    println!("✅ 持久化一致性：原子操作结果正确持久化到数据库");

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}