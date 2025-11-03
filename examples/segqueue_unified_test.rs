use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 SegQueue统一架构测试");
    println!("=========================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("segqueue_unified_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建统一路由器");
    println!("-----------------------");

    let manager = AtomicOperationsManager::new(db.clone());
    let manager = Arc::new(manager);
    println!("  ✅ 统一路由器创建成功");

    println!("\n📋 测试2: 基础路由功能");
    println!("-------------------");

    // 测试原子递增路由
    let val1 = manager.increment("test_counter".to_string(), 1)?;
    println!("  原子递增路由: {}", val1);

    let val2 = manager.increment("test_counter".to_string(), 2)?;
    println!("  原子递增路由: {}", val2);

    // 测试数据库操作路由
    manager.insert(b"test:key1", "value1".as_bytes())?;
    println!("  数据库插入路由成功");

    let retrieved = manager.get_data(b"test:key1")?;
    println!("  数据库获取路由: {:?}", retrieved.map(|v| String::from_utf8(v.to_vec()).unwrap_or_else(|_| "无效UTF8".to_string())));

    println!("\n📋 测试3: Worker间通信测试");
    println!("-------------------------");

    // 测试AtomicWorker自动向DatabaseWorker发送持久化指令
    let counter_val = manager.increment("auto_persist_test".to_string(), 5)?;
    println!("  原子操作完成，值: {}", counter_val);

    // 等待一下让持久化操作完成
    thread::sleep(std::time::Duration::from_millis(50));

    // 创建新的管理器验证持久化
    let manager2 = AtomicOperationsManager::new(db.clone());
    let loaded_count = manager2.preload_counters()?;
    println!("  预热加载计数器数量: {}", loaded_count);

    let persisted_val = manager2.get("auto_persist_test".to_string())?;
    println!("  持久化验证: {:?} (原: {})", persisted_val, counter_val);

    if persisted_val == Some(counter_val) {
        println!("  ✅ Worker间通信测试通过");
    } else {
        println!("  ❌ Worker间通信测试失败");
    }

    println!("\n📋 测试4: 6线程高并发混合测试");
    println!("----------------------------");

    let mut handles = vec![];

    // 线程1：纯原子操作
    let manager_clone1 = Arc::clone(&manager);
    let handle1 = thread::spawn(move || {
        for i in 0..30 {
            match manager_clone1.increment("concurrent_atomic".to_string(), 1) {
                Ok(value) => {
                    if i % 10 == 0 {
                        println!("  线程1(原子): 计数器 = {}", value);
                    }
                }
                Err(e) => eprintln!("  线程1原子操作失败: {:?}", e),
            }
        }
    });

    // 线程2：纯原子操作
    let manager_clone2 = Arc::clone(&manager);
    let handle2 = thread::spawn(move || {
        for i in 0..25 {
            match manager_clone2.increment("concurrent_atomic".to_string(), 2) {
                Ok(value) => {
                    if i % 8 == 0 {
                        println!("  线程2(原子): 计数器 = {}", value);
                    }
                }
                Err(e) => eprintln!("  线程2原子操作失败: {:?}", e),
            }
        }
    });

    // 线程3：纯数据库操作
    let manager_clone3 = Arc::clone(&manager);
    let handle3 = thread::spawn(move || {
        for i in 0..20 {
            let key = format!("db_test:item:{}", i);
            let value = format!("data_value_{}", i);
            if let Err(e) = manager_clone3.insert(key.as_bytes(), value.as_bytes()) {
                eprintln!("  线程3数据库插入失败: {:?}", e);
            }
            if i % 7 == 0 {
                println!("  线程3(数据库): 插入项 {}", i);
            }
        }
    });

    // 线程4：混合操作（先原子后数据库）
    let manager_clone4 = Arc::clone(&manager);
    let handle4 = thread::spawn(move || {
        for i in 0..15 {
            // 先进行原子操作
            match manager_clone4.increment("mixed_counter".to_string(), 1) {
                Ok(user_id) => {
                    // 然后进行数据库操作
                    let key = format!("user:{}", user_id);
                    let value = format!("用户{}", i);
                    if let Err(e) = manager_clone4.insert(key.as_bytes(), value.as_bytes()) {
                        eprintln!("  线程4用户创建失败: {:?}", e);
                    }
                    if i % 5 == 0 {
                        println!("  线程4(混合): 创建用户{} ID:{}", i, user_id);
                    }
                }
                Err(e) => eprintln!("  线程4原子操作失败: {:?}", e),
            }
        }
    });

    // 线程5：数据库读取操作
    let manager_clone5 = Arc::clone(&manager);
    let handle5 = thread::spawn(move || {
        for i in 0..12 {
            // 读取数据库数据
            let scan_results = manager_clone5.scan_prefix(b"db_test:");
            match scan_results {
                Ok(items) => {
                    if i % 4 == 0 {
                        println!("  线程5(读取): 找到 {} 条数据", items.len());
                    }
                }
                Err(e) => eprintln!("  线程5扫描失败: {:?}", e),
            }

            thread::sleep(std::time::Duration::from_millis(15));
        }
    });

    // 线程6：页面访问计数
    let manager_clone6 = Arc::clone(&manager);
    let handle6 = thread::spawn(move || {
        for i in 0..20 {
            match manager_clone6.increment("page_views".to_string(), 1) {
                Ok(count) => {
                    if i % 7 == 0 {
                        println!("  线程6(访问): 页面访问量 = {}", count);
                    }
                }
                Err(e) => eprintln!("  线程6访问计数失败: {:?}", e),
            }
        }
    });

    handles.push(handle1);
    handles.push(handle2);
    handles.push(handle3);
    handles.push(handle4);
    handles.push(handle5);
    handles.push(handle6);

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n📋 测试5: 结果验证");
    println!("-----------------");

    // 验证原子计数器
    let atomic_val = manager.get("concurrent_atomic".to_string())?;
    let mixed_val = manager.get("mixed_counter".to_string())?;
    let page_views_val = manager.get("page_views".to_string())?;

    println!("  原子计数器验证:");
    println!("    concurrent_atomic: {:?}", atomic_val);
    println!("    mixed_counter: {:?}", mixed_val);
    println!("    page_views: {:?}", page_views_val);

    // 验证数据库记录
    let db_records = manager.scan_prefix(b"db_test:")?;
    let user_records = manager.scan_prefix(b"user:")?;

    println!("  数据库记录验证:");
    println!("    db_test 记录数: {}", db_records.len());
    println!("    user 记录数: {}", user_records.len());

    // 验证预期值
    let expected_atomic = 30 * 1 + 25 * 2; // 线程1: 30*1, 线程2: 25*2 = 80
    let expected_mixed = 15; // 线程4创建了15个用户
    let expected_page_views = 20; // 线程6访问了20次

    let atomic_ok = atomic_val == Some(expected_atomic);
    let mixed_ok = mixed_val == Some(expected_mixed);
    let page_views_ok = page_views_val == Some(expected_page_views);
    let db_records_ok = db_records.len() == 20;
    let user_records_ok = user_records.len() == 15;

    println!("\n📋 测试6: 最终持久化验证");
    println!("-----------------------");

    // 等待所有持久化操作完成
    thread::sleep(std::time::Duration::from_millis(100));

    // 创建新管理器验证最终持久化
    let final_manager = AtomicOperationsManager::new(db.clone());
    let final_loaded = final_manager.preload_counters()?;
    println!("  最终预热计数器数量: {}", final_loaded);

    let final_atomic = final_manager.get("concurrent_atomic".to_string())?;
    let final_mixed = final_manager.get("mixed_counter".to_string())?;
    let final_page_views = final_manager.get("page_views".to_string())?;

    println!("  最终持久化验证:");
    println!("    concurrent_atomic: {:?} (原: {:?})", final_atomic, atomic_val);
    println!("    mixed_counter: {:?} (原: {:?})", final_mixed, mixed_val);
    println!("    page_views: {:?} (原: {:?})", final_page_views, page_views_val);

    let persistence_ok = final_atomic == atomic_val &&
                        final_mixed == mixed_val &&
                        final_page_views == page_views_val;

    println!("\n🎉 测试完成！");
    println!("=============");

    let all_ok = atomic_ok && mixed_ok && page_views_ok &&
                 db_records_ok && user_records_ok && persistence_ok;

    if all_ok {
        println!("✅ SegQueue统一架构测试完全通过");
        println!("✅ 纯路由器设计成功");
        println!("✅ Worker间通信正常");
        println!("✅ 原子操作自动持久化有效");
        println!("✅ 6线程高并发混合操作安全");
        println!("✅ 无EBR冲突");
        println!("✅ 数据一致性保证");
    } else {
        println!("❌ 部分测试失败:");
        if !atomic_ok { println!("  - atomic_counter失败: 预期{}, 实际{:?}", expected_atomic, atomic_val); }
        if !mixed_ok { println!("  - mixed_counter失败: 预期{}, 实际{:?}", expected_mixed, mixed_val); }
        if !page_views_ok { println!("  - page_views失败: 预期{}, 实际{:?}", expected_page_views, page_views_val); }
        if !db_records_ok { println!("  - db_records失败: 预期20, 实际{}", db_records.len()); }
        if !user_records_ok { println!("  - user_records失败: 预期15, 实际{}", user_records.len()); }
        if !persistence_ok { println!("  - 最终持久化验证失败"); }
    }

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}