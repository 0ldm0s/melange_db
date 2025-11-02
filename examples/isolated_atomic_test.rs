use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::sync::mpsc;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 完全分离原子操作测试");
    println!("========================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("isolated_atomic_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建管理器");
    println!("-------------------");

    let manager = AtomicOperationsManager::new(db.clone());
    let manager = Arc::new(manager);
    println!("  ✅ 管理器创建成功");

    println!("\n📋 测试2: 完全分离的并发测试");
    println!("-------------------------");

    // 使用屏障进行同步
    use std::sync::Barrier;
    let barrier = Arc::new(Barrier::new(6));
    let (done_tx, done_rx) = mpsc::channel();

    let mut handles = vec![];

    // 线程1：纯原子操作
    let manager_clone1 = Arc::clone(&manager);
    let barrier1 = Arc::clone(&barrier);
    let done_tx1 = done_tx.clone();
    let handle1 = thread::spawn(move || {
        barrier1.wait(); // 等待所有线程准备就绪

        for i in 0..25 {
            match manager_clone1.increment("atomic_counter".to_string(), 1) {
                Ok(value) => {
                    if i % 8 == 0 {
                        println!("  线程1(原子操作): 计数器 = {}", value);
                    }
                }
                Err(e) => eprintln!("  线程1原子操作失败: {:?}", e),
            }
        }

        done_tx1.send("thread1_done").unwrap();
    });

    // 线程2：纯原子操作
    let manager_clone2 = Arc::clone(&manager);
    let barrier2 = Arc::clone(&barrier);
    let done_tx2 = done_tx.clone();
    let handle2 = thread::spawn(move || {
        barrier2.wait(); // 等待所有线程准备就绪

        for i in 0..20 {
            match manager_clone2.increment("atomic_counter".to_string(), 2) {
                Ok(value) => {
                    if i % 6 == 0 {
                        println!("  线程2(原子操作): 计数器 = {}", value);
                    }
                }
                Err(e) => eprintln!("  线程2原子操作失败: {:?}", e),
            }
        }

        done_tx2.send("thread2_done").unwrap();
    });

    // 线程3：纯数据库操作
    let db_clone3 = Arc::clone(&db);
    let barrier3 = Arc::clone(&barrier);
    let done_tx3 = done_tx.clone();
    let handle3 = thread::spawn(move || {
        barrier3.wait(); // 等待所有线程准备就绪

        for i in 0..18 {
            let key = format!("db_data:item:{}", i);
            let value = format!("database_value_{}", i);
            if let Err(e) = db_clone3.insert(key.as_bytes(), value.as_bytes()) {
                eprintln!("  线程3数据库写入失败: {:?}", e);
            }
            if i % 6 == 0 {
                println!("  线程3(数据库操作): 写入项 {}", i);
            }
        }

        done_tx3.send("thread3_done").unwrap();
    });

    // 线程4：纯数据库操作（读取）
    let db_clone4 = Arc::clone(&db);
    let barrier4 = Arc::clone(&barrier);
    let done_tx4 = done_tx.clone();
    let handle4 = thread::spawn(move || {
        barrier4.wait(); // 等待所有线程准备就绪

        for i in 0..12 {
            let count = db_clone4.scan_prefix(b"db_data:").count();
            if i % 4 == 0 {
                println!("  线程4(数据库读取): 当前数据项数 = {}", count);
            }
            thread::sleep(std::time::Duration::from_millis(8));
        }

        done_tx4.send("thread4_done").unwrap();
    });

    // 线程5：纯原子操作（另一个计数器）
    let manager_clone5 = Arc::clone(&manager);
    let barrier5 = Arc::clone(&barrier);
    let done_tx5 = done_tx.clone();
    let handle5 = thread::spawn(move || {
        barrier5.wait(); // 等待所有线程准备就绪

        for i in 0..15 {
            match manager_clone5.increment("page_views".to_string(), 1) {
                Ok(value) => {
                    if i % 5 == 0 {
                        println!("  线程5(页面访问): 访问量 = {}", value);
                    }
                }
                Err(e) => eprintln!("  线程5页面访问计数失败: {:?}", e),
            }
        }

        done_tx5.send("thread5_done").unwrap();
    });

    // 线程6：混合操作 - 但在不同时间点进行
    let manager_clone6 = Arc::clone(&manager);
    let db_clone6 = Arc::clone(&db);
    let barrier6 = Arc::clone(&barrier);
    let done_tx6 = done_tx.clone();
    let handle6 = thread::spawn(move || {
        barrier6.wait(); // 等待所有线程准备就绪

        // 阶段1：先进行原子操作
        for i in 0..8 {
            match manager_clone6.increment("user_counter".to_string(), 1) {
                Ok(user_id) => {
                    if i % 3 == 0 {
                        println!("  线程6(混合-原子): 用户ID = {}", user_id);
                    }
                }
                Err(e) => eprintln!("  线程6用户ID分配失败: {:?}", e),
            }
        }

        // 短暂暂停
        thread::sleep(std::time::Duration::from_millis(20));

        // 阶段2：然后进行数据库操作
        for i in 0..6 {
            let key = format!("mixed_data:item:{}", i);
            let value = format!("mixed_value_{}", i);
            if let Err(e) = db_clone6.insert(key.as_bytes(), value.as_bytes()) {
                eprintln!("  线程6数据库写入失败: {:?}", e);
            }
            if i % 2 == 0 {
                println!("  线程6(混合-数据库): 写入混合项 {}", i);
            }
        }

        done_tx6.send("thread6_done").unwrap();
    });

    handles.push(handle1);
    handles.push(handle2);
    handles.push(handle3);
    handles.push(handle4);
    handles.push(handle5);
    handles.push(handle6);

    // 释放发送端
    drop(done_tx);

    println!("  启动所有线程...");

    for handle in handles {
        handle.join().unwrap();
    }

    // 等待所有线程完成
    for _ in 0..6 {
        let done_msg = done_rx.recv().unwrap();
        println!("  收到完成信号: {}", done_msg);
    }

    println!("\n📋 测试3: 验证结果");
    println!("-----------------");

    // 验证原子计数器
    let atomic_counter = manager.get("atomic_counter".to_string())?;
    let page_views = manager.get("page_views".to_string())?;
    let user_counter = manager.get("user_counter".to_string())?;

    println!("  原子计数器结果:");
    println!("    atomic_counter: {:?}", atomic_counter);
    println!("    page_views: {:?}", page_views);
    println!("    user_counter: {:?}", user_counter);

    // 验证数据库记录
    let db_data_count = db.scan_prefix(b"db_data:").count();
    let mixed_data_count = db.scan_prefix(b"mixed_data:").count();

    println!("  数据库记录结果:");
    println!("    db_data 记录数: {}", db_data_count);
    println!("    mixed_data 记录数: {}", mixed_data_count);

    // 验证计数器一致性
    let expected_atomic = 25 * 1 + 20 * 2; // 线程1: 25*1, 线程2: 20*2 = 65
    let atomic_ok = atomic_counter == Some(expected_atomic);
    let page_views_ok = page_views == Some(15);
    let user_counter_ok = user_counter == Some(8);
    let db_data_ok = db_data_count == 18;
    let mixed_data_ok = mixed_data_count == 6;

    println!("\n📋 测试4: 持久化测试");
    println!("------------------");

    // 在所有操作完成后进行持久化
    println!("  开始持久化所有计数器...");
    let persisted_count = manager.persist_all_counters()?;
    println!("  持久化了 {} 个计数器", persisted_count);

    // 验证持久化
    let manager2 = AtomicOperationsManager::new(db.clone());
    let loaded_count = manager2.preload_counters()?;
    println!("  新管理器加载了 {} 个计数器", loaded_count);

    let reloaded_atomic = manager2.get("atomic_counter".to_string())?;
    let reloaded_page_views = manager2.get("page_views".to_string())?;
    let reloaded_user_counter = manager2.get("user_counter".to_string())?;

    println!("  重新加载的计数器:");
    println!("    atomic_counter: {:?}", reloaded_atomic);
    println!("    page_views: {:?}", reloaded_page_views);
    println!("    user_counter: {:?}", reloaded_user_counter);

    let persistence_ok = reloaded_atomic == atomic_counter &&
                        reloaded_page_views == page_views &&
                        reloaded_user_counter == user_counter;

    println!("\n🎉 测试完成！");
    println!("=============");

    let all_ok = atomic_ok && page_views_ok && user_counter_ok &&
                 db_data_ok && mixed_data_ok && persistence_ok;

    if all_ok {
        println!("✅ 所有测试通过");
        println!("✅ 纯原子操作正常");
        println!("✅ 纯数据库操作正常");
        println!("✅ 分阶段的混合操作正常");
        println!("✅ 6线程并发安全");
        println!("✅ 持久化机制有效");
        println!("✅ 无EBR冲突");
    } else {
        println!("❌ 部分测试失败:");
        if !atomic_ok { println!("  - atomic_counter失败: 预期{}, 实际{:?}", expected_atomic, atomic_counter); }
        if !page_views_ok { println!("  - page_views失败: 预期15, 实际{:?}", page_views); }
        if !user_counter_ok { println!("  - user_counter失败: 预期8, 实际{:?}", user_counter); }
        if !db_data_ok { println!("  - db_data_count失败: 预期18, 实际{}", db_data_count); }
        if !mixed_data_ok { println!("  - mixed_data_count失败: 预期6, 实际{}", mixed_data_count); }
        if !persistence_ok { println!("  - 持久化验证失败"); }
    }

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}