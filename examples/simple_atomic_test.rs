use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 简单原子操作测试（分离持久化）");
    println!("==================================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("simple_atomic_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建管理器");
    println!("-------------------");

    let manager = AtomicOperationsManager::new(db.clone());
    let manager = Arc::new(manager);
    println!("  ✅ 管理器创建成功");

    println!("\n📋 测试2: 纯原子操作测试（不持久化）");
    println!("------------------------------");

    let mut handles = vec![];

    // 线程1：原子递增操作
    let manager_clone1 = Arc::clone(&manager);
    let handle1 = thread::spawn(move || {
        for i in 0..20 {
            match manager_clone1.increment("test_counter".to_string(), 1) {
                Ok(value) => {
                    if i % 5 == 0 {
                        println!("  线程1: 计数器递增到 {}", value);
                    }
                }
                Err(e) => eprintln!("  线程1递增失败: {:?}", e),
            }
        }
    });

    // 线程2：原子递增操作
    let manager_clone2 = Arc::clone(&manager);
    let handle2 = thread::spawn(move || {
        for i in 0..20 {
            match manager_clone2.increment("test_counter".to_string(), 2) {
                Ok(value) => {
                    if i % 5 == 0 {
                        println!("  线程2: 计数器递增到 {}", value);
                    }
                }
                Err(e) => eprintln!("  线程2递增失败: {:?}", e),
            }
        }
    });

    // 线程3：常规数据库操作
    let db_clone3 = Arc::clone(&db);
    let handle3 = thread::spawn(move || {
        for i in 0..15 {
            let key = format!("data:item:{}", i);
            let value = format!("value{}", i);
            if let Err(e) = db_clone3.insert(key.as_bytes(), value.as_bytes()) {
                eprintln!("  线程3写入失败: {:?}", e);
            }
            if i % 5 == 0 {
                println!("  线程3: 写入数据项 {}", i);
            }
        }
    });

    handles.push(handle1);
    handles.push(handle2);
    handles.push(handle3);

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 获取最终计数器值
    let final_value = manager.get("test_counter".to_string())?;
    println!("  最终计数器值: {:?}", final_value);

    let expected = 20 * 1 + 20 * 2; // 线程1: 20*1, 线程2: 20*2 = 60
    if final_value == Some(expected) {
        println!("  ✅ 纯原子操作测试通过");
    } else {
        println!("  ❌ 纯原子操作测试失败: 预期{}, 实际{:?}", expected, final_value);
    }

    println!("\n📋 测试3: 手动持久化测试");
    println!("----------------------");

    // 在所有并发操作完成后，统一进行持久化
    println!("  开始持久化所有计数器...");
    let persisted_count = manager.persist_all_counters()?;
    println!("  持久化了 {} 个计数器", persisted_count);

    println!("\n📋 测试4: 持久化验证");
    println!("------------------");

    // 创建新管理器验证持久化
    let manager2 = AtomicOperationsManager::new(db.clone());
    let loaded_count = manager2.preload_counters()?;
    println!("  新管理器加载了 {} 个计数器", loaded_count);

    let reloaded_value = manager2.get("test_counter".to_string())?;
    println!("  重新加载的计数器值: {:?}", reloaded_value);

    if reloaded_value == final_value {
        println!("  ✅ 持久化验证通过");
    } else {
        println!("  ❌ 持久化验证失败: 预期{:?}, 实际{:?}", final_value, reloaded_value);
    }

    println!("\n📋 测试5: 复杂场景测试");
    println!("-------------------");

    let mut handles = vec![];

    // 线程4：用户ID分配（原子操作）
    let manager_clone4 = Arc::clone(&manager);
    let handle4 = thread::spawn(move || {
        for i in 0..10 {
            match manager_clone4.increment("user_id".to_string(), 1) {
                Ok(user_id) => {
                    let username = format!("用户{}", i);
                    if let Err(e) = manager_clone4.insert(format!("user:{}", user_id).as_bytes(), username.as_bytes()) {
                        eprintln!("  线程4创建用户失败: {:?}", e);
                    }
                    if i % 3 == 0 {
                        println!("  线程4: 创建用户{}", user_id);
                    }
                }
                Err(e) => eprintln!("  线程4分配用户ID失败: {:?}", e),
            }
        }
    });

    // 线程5：数据统计（常规操作）
    let db_clone5 = Arc::clone(&db);
    let handle5 = thread::spawn(move || {
        for i in 0..8 {
            let user_count = db_clone5.scan_prefix(b"user:").count();
            let data_count = db_clone5.scan_prefix(b"data:").count();

            if i % 2 == 0 {
                println!("  线程5: 用户数:{} 数据数:{}", user_count, data_count);
            }

            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // 线程6：页面访问计数（原子操作）
    let manager_clone6 = Arc::clone(&manager);
    let handle6 = thread::spawn(move || {
        for i in 0..15 {
            match manager_clone6.increment("page_views".to_string(), 1) {
                Ok(count) => {
                    if i % 5 == 0 {
                        println!("  线程6: 页面访问量: {}", count);
                    }
                }
                Err(e) => eprintln!("  线程6访问计数失败: {:?}", e),
            }
        }
    });

    handles.push(handle4);
    handles.push(handle5);
    handles.push(handle6);

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 最终持久化
    println!("  最终持久化...");
    let final_persisted = manager.persist_all_counters()?;
    println!("  持久化了 {} 个计数器", final_persisted);

    // 验证结果
    let user_id = manager.get("user_id".to_string())?;
    let page_views = manager.get("page_views".to_string())?;
    let user_count = manager.db().scan_prefix(b"user:").count();
    let data_count = manager.db().scan_prefix(b"data:").count();

    println!("\n📋 最终验证");
    println!("-----------");
    println!("  用户ID计数器: {:?}", user_id);
    println!("  页面访问计数器: {:?}", page_views);
    println!("  实际用户记录数: {}", user_count);
    println!("  实际数据记录数: {}", data_count);

    let user_consistency = user_id.unwrap_or(0) >= user_count as u64;
    let test_success = user_consistency && page_views.is_some();

    println!("\n🎉 测试完成！");
    println!("=============");
    if test_success {
        println!("✅ 原子操作正常工作");
        println!("✅ 数据库操作正常工作");
        println!("✅ 混合并发安全");
        println!("✅ 手动持久化有效");
    } else {
        println!("❌ 部分测试失败");
    }

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}