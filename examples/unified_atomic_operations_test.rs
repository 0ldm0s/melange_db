use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 统一入口原子操作混合测试");
    println!("==============================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("unified_atomic_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建统一入口管理器");
    println!("------------------------------");

    // 创建AtomicOperationsManager
    let manager = AtomicOperationsManager::new(db.clone());
    let manager = Arc::new(manager);
    println!("  ✅ AtomicOperationsManager创建成功");

    // 预热计数器
    let loaded_count = manager.preload_counters()?;
    println!("  预热加载了 {} 个计数器", loaded_count);

    println!("\n📋 测试2: 基本原子操作");
    println!("--------------------");

    // 测试原子递增
    let val1 = manager.increment("test_counter".to_string(), 1)?;
    println!("  第1次递增: {}", val1);

    let val2 = manager.increment("test_counter".to_string(), 1)?;
    println!("  第2次递增: {}", val2);

    let val3 = manager.increment("test_counter".to_string(), 5)?;
    println!("  步长5递增: {}", val3);

    let current = manager.get("test_counter".to_string())?;
    println!("  当前计数器值: {:?}", current);

    if current == Some(7) {
        println!("  ✅ 基本原子操作测试通过");
    } else {
        println!("  ❌ 基本原子操作测试失败: 预期7，实际{:?}", current);
    }

    println!("\n📋 测试3: 基本数据库操作");
    println!("----------------------");

    // 测试常规数据库操作
    manager.insert(b"user:1001", "张三".as_bytes())?;
    manager.insert(b"user:1002", "李四".as_bytes())?;
    println!("  ✅ 插入用户数据");

    let user = manager.get_data(b"user:1001")?;
    println!("  用户1001: {:?}", user.map(|v| String::from_utf8(v.to_vec()).unwrap_or_else(|_| "无效UTF8".to_string())));

    println!("\n📋 测试4: 6线程混合并发测试");
    println!("-------------------------");

    let mut handles = vec![];

    // 线程1：用户ID原子分配
    let manager_clone1 = Arc::clone(&manager);
    let handle1 = thread::spawn(move || {
        for i in 0..15 {
            match manager_clone1.increment("user_id_counter".to_string(), 1) {
                Ok(user_id) => {
                    // 使用分配的ID创建用户
                    let username = format!("线程1用户{}", i);
                    if let Err(e) = manager_clone1.insert(format!("user:{}", user_id).as_bytes(), username.as_bytes()) {
                        eprintln!("  线程1写入用户失败: {:?}", e);
                    }
                    if i % 5 == 0 {
                        println!("  线程1(用户分配): 创建用户{}", user_id);
                    }
                }
                Err(e) => eprintln!("  线程1分配用户ID失败: {:?}", e),
            }
        }
    });

    // 线程2：订单ID原子分配
    let manager_clone2 = Arc::clone(&manager);
    let handle2 = thread::spawn(move || {
        for i in 0..15 {
            match manager_clone2.increment("order_counter".to_string(), 1) {
                Ok(order_id) => {
                    // 使用分配的ID创建订单
                    let product = format!("产品{}", i % 3);
                    if let Err(e) = manager_clone2.insert(format!("order:{}", order_id).as_bytes(), product.as_bytes()) {
                        eprintln!("  线程2写入订单失败: {:?}", e);
                    }
                    if i % 5 == 0 {
                        println!("  线程2(订单分配): 创建订单{}", order_id);
                    }
                }
                Err(e) => eprintln!("  线程2分配订单ID失败: {:?}", e),
            }
        }
    });

    // 线程3：页面访问原子计数
    let manager_clone3 = Arc::clone(&manager);
    let handle3 = thread::spawn(move || {
        for i in 0..25 {
            match manager_clone3.increment("page_views_counter".to_string(), 1) {
                Ok(count) => {
                    if i % 8 == 0 {
                        println!("  线程3(访问计数): 页面访问数: {}", count);
                    }
                }
                Err(e) => eprintln!("  线程3访问计数失败: {:?}", e),
            }
        }
    });

    // 线程4：数据读取操作
    let manager_clone4 = Arc::clone(&manager);
    let handle4 = thread::spawn(move || {
        for i in 0..12 {
            // 读取用户数据
            let user_count = manager_clone4.db().scan_prefix(b"user:").count();
            let order_count = manager_clone4.db().scan_prefix(b"order:").count();

            match manager_clone4.increment("read_operation_counter".to_string(), 1) {
                Ok(read_count) => {
                    if i % 4 == 0 {
                        println!("  线程4(数据读取): 用户数:{} 订单数:{} 读操作:{}", user_count, order_count, read_count);
                    }
                }
                Err(e) => eprintln!("  线程4读操作计数失败: {:?}", e),
            }

            // 短暂休眠
            thread::sleep(std::time::Duration::from_millis(5));
        }
    });

    // 线程5：批量数据写入
    let manager_clone5 = Arc::clone(&manager);
    let handle5 = thread::spawn(move || {
        for i in 0..10 {
            let key = format!("batch:item:{}", i);
            let value = format!("批量数据{}", i);
            if let Err(e) = manager_clone5.insert(key.as_bytes(), value.as_bytes()) {
                eprintln!("  线程5批量写入失败: {:?}", e);
            }

            // 原子计数器记录写入次数
            match manager_clone5.increment("batch_write_counter".to_string(), 1) {
                Ok(write_count) => {
                    if i % 3 == 0 {
                        println!("  线程5(批量写入): 写入项目{} 总写入次数:{}", i, write_count);
                    }
                }
                Err(e) => eprintln!("  线程5写入计数失败: {:?}", e),
            }
        }
    });

    // 线程6：统计和监控
    let manager_clone6 = Arc::clone(&manager);
    let handle6 = thread::spawn(move || {
        for i in 0..8 {
            // 获取各种计数器
            let user_id_counter = manager_clone6.get("user_id_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let order_counter = manager_clone6.get("order_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let page_views = manager_clone6.get("page_views_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);

            // 统计数据库记录
            let total_records = manager_clone6.db().scan_prefix(b"").count();

            // 记录统计次数
            match manager_clone6.increment("statistics_counter".to_string(), 1) {
                Ok(stat_count) => {
                    if i % 2 == 0 {
                        println!("  线程6(统计监控): 用户ID:{} 订单ID:{} 访问量:{} 总记录:{} 统计次数:{}",
                                user_id_counter, order_counter, page_views, total_records, stat_count);
                    }
                }
                Err(e) => eprintln!("  线程6统计计数失败: {:?}", e),
            }

            thread::sleep(std::time::Duration::from_millis(15));
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

    println!("\n📋 测试5: 数据一致性验证");
    println!("-----------------------");

    // 验证原子计数器
    let user_counter = manager.get("user_id_counter".to_string())?;
    let order_counter = manager.get("order_counter".to_string())?;
    let page_views = manager.get("page_views_counter".to_string())?;
    let read_ops = manager.get("read_operation_counter".to_string())?;
    let batch_writes = manager.get("batch_write_counter".to_string())?;
    let stats = manager.get("statistics_counter".to_string())?;

    println!("  原子计数器验证:");
    println!("    用户ID计数器: {:?}", user_counter);
    println!("    订单计数器: {:?}", order_counter);
    println!("    页面访问数: {:?}", page_views);
    println!("    读操作计数: {:?}", read_ops);
    println!("    批量写入计数: {:?}", batch_writes);
    println!("    统计操作计数: {:?}", stats);

    // 验证实际数据
    let user_count = manager.db().scan_prefix(b"user:").count();
    let order_count = manager.db().scan_prefix(b"order:").count();
    let batch_count = manager.db().scan_prefix(b"batch:").count();

    println!("  实际数据验证:");
    println!("    用户记录数: {}", user_count);
    println!("    订单记录数: {}", order_count);
    println!("    批量记录数: {}", batch_count);

    // 验证数据一致性
    let user_consistency = user_counter.unwrap_or(0) >= user_count as u64;
    let order_consistency = order_counter.unwrap_or(0) >= order_count as u64;
    let batch_consistency = batch_writes.unwrap_or(0) == batch_count as u64;

    println!("\n📋 测试6: 持久化验证");
    println!("------------------");

    // 创建新的管理器实例测试持久化
    let manager2 = AtomicOperationsManager::new(db.clone());
    let reloaded_count = manager2.preload_counters()?;
    println!("  新管理器预热加载了 {} 个计数器", reloaded_count);

    // 验证持久化的数据
    let persisted_user_counter = manager2.get("user_id_counter".to_string())?;
    let persisted_order_counter = manager2.get("order_counter".to_string())?;
    let persisted_page_views = manager2.get("page_views_counter".to_string())?;

    println!("  持久化验证:");
    println!("    用户计数器: {:?} (原: {:?})", persisted_user_counter, user_counter);
    println!("    订单计数器: {:?} (原: {:?})", persisted_order_counter, order_counter);
    println!("    页面访问数: {:?} (原: {:?})", persisted_page_views, page_views);

    let persistence_ok = persisted_user_counter == user_counter &&
                        persisted_order_counter == order_counter &&
                        persisted_page_views == page_views;

    println!("\n🎉 统一入口混合测试完成！");
    println!("========================");

    if user_consistency && order_consistency && batch_consistency && persistence_ok {
        println!("✅ 所有测试通过");
        println!("✅ 原子操作正常工作");
        println!("✅ 数据库操作正常工作");
        println!("✅ 混合并发操作安全");
        println!("✅ 数据一致性保证");
        println!("✅ 持久化机制有效");
    } else {
        println!("❌ 部分测试失败:");
        if !user_consistency { println!("  - 用户数据一致性验证失败"); }
        if !order_consistency { println!("  - 订单数据一致性验证失败"); }
        if !batch_consistency { println!("  - 批量写入一致性验证失败"); }
        if !persistence_ok { println!("  - 持久化验证失败"); }
    }

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}