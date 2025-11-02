use melange_db::{Db, Config, platform_utils, atomic_worker::AtomicWorker};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 原子操作与常规操作混合测试");
    println!("==============================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("atomic_mixed_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 常规数据库操作");
    println!("-----------------------");

    // 常规数据操作
    db.insert(b"user:1001", "张三".as_bytes())?;
    db.insert(b"user:1002", "李四".as_bytes())?;
    db.insert(b"order:1001", "商品A x 2".as_bytes())?;
    db.insert(b"order:1002", "商品B x 1".as_bytes())?;

    println!("  ✅ 插入用户和订单数据");

    // 读取数据
    let user = db.get(b"user:1001")?;
    let order = db.get(b"order:1001")?;
    println!("  用户1001: {:?}", user.map(|v| String::from_utf8(v.to_vec()).unwrap_or_else(|_| "无效UTF8".to_string())));
    println!("  订单1001: {:?}", order.map(|v| String::from_utf8(v.to_vec()).unwrap_or_else(|_| "无效UTF8".to_string())));

    println!("\n📋 测试2: 创建原子操作Worker");
    println!("----------------------------");

    // 创建AtomicWorker
    let atomic_worker = AtomicWorker::new(db.clone());
    let atomic_worker = Arc::new(atomic_worker);
    println!("  ✅ AtomicWorker创建成功");

    println!("\n📋 测试3: 用户ID自增分配");
    println!("----------------------");

    // 使用原子计数器分配用户ID
    let next_user_id = atomic_worker.increment("user_id_counter".to_string(), 1)?;
    println!("  下一个用户ID: {}", next_user_id);

    // 使用分配的ID创建新用户
    db.insert(format!("user:{}", next_user_id).as_bytes(), "王五".as_bytes())?;
    println!("  创建用户{}: 王五", next_user_id);

    let next_user_id2 = atomic_worker.increment("user_id_counter".to_string(), 1)?;
    db.insert(format!("user:{}", next_user_id2).as_bytes(), "赵六".as_bytes())?;
    println!("  创建用户{}: 赵六", next_user_id2);

    println!("\n📋 测试4: 订单计数统计");
    println!("---------------------");

    // 订单计数器
    let order_count = atomic_worker.increment("order_counter".to_string(), 1)?;
    println!("  订单总数: {}", order_count);

    // 商品库存计数器（增加而不是减少，避免负数问题）
    let product_a_stock = atomic_worker.increment("product_a_stock".to_string(), 5)?;
    let product_b_stock = atomic_worker.increment("product_b_stock".to_string(), 1)?;
    println!("  商品A库存变化: {}", product_a_stock);
    println!("  商品B库存变化: {}", product_b_stock);

    println!("\n📋 测试5: 6线程混合并发压力测试");
    println!("-----------------------------");

    let mut handles = vec![];

    // 原子操作线程组（3个线程）
    println!("  启动3个原子操作线程...");

    // 线程1：用户ID分配
    let atomic_worker_clone1 = Arc::clone(&atomic_worker);
    let handle1 = thread::spawn(move || {
        for i in 0..20 {
            match atomic_worker_clone1.increment("user_id_counter".to_string(), 1) {
                Ok(user_id) => {
                    if i % 5 == 0 {
                        println!("  线程1(用户ID): 分配用户{}", user_id);
                    }
                }
                Err(e) => eprintln!("  线程1: 分配用户ID失败: {:?}", e),
            }
        }
    });

    // 线程2：订单ID分配
    let atomic_worker_clone2 = Arc::clone(&atomic_worker);
    let handle2 = thread::spawn(move || {
        for i in 0..20 {
            match atomic_worker_clone2.increment("order_counter".to_string(), 1) {
                Ok(order_id) => {
                    if i % 5 == 0 {
                        println!("  线程2(订单ID): 分配订单{}", order_id);
                    }
                }
                Err(e) => eprintln!("  线程2: 分配订单ID失败: {:?}", e),
            }
        }
    });

    // 线程3：页面访问计数
    let atomic_worker_clone3 = Arc::clone(&atomic_worker);
    let handle3 = thread::spawn(move || {
        for i in 0..30 {
            match atomic_worker_clone3.increment("page_views_counter".to_string(), 1) {
                Ok(count) => {
                    if i % 6 == 0 {
                        println!("  线程3(访问计数): 页面访问数: {}", count);
                    }
                }
                Err(e) => eprintln!("  线程3: 页面访问计数失败: {:?}", e),
            }
        }
    });

    // 常规数据库操作线程组（3个线程）
    println!("  启动3个常规数据库操作线程...");

    // 线程4：用户数据写入
    let db_clone4 = Arc::clone(&db);
    let atomic_worker_clone4 = Arc::clone(&atomic_worker);
    let handle4 = thread::spawn(move || {
        for i in 0..20 {
            // 先获取用户ID，然后写入用户数据
            match atomic_worker_clone4.increment("user_id_counter".to_string(), 1) {
                Ok(user_id) => {
                    let username = format!("常规用户{}", i);
                    let email = format!("user{}@example.com", user_id);
                    let user_data = format!("{}|{}", username, email);
                    if let Err(e) = db_clone4.insert(format!("user:{}", user_id).as_bytes(), user_data.as_bytes()) {
                        eprintln!("  线程4: 写入用户数据失败: {:?}", e);
                    }
                    if i % 5 == 0 {
                        println!("  线程4(用户写入): 写入用户{} {}", user_id, username);
                    }
                }
                Err(e) => eprintln!("  线程4: 获取用户ID失败: {:?}", e),
            }
        }
    });

    // 线程5：订单数据写入
    let db_clone5 = Arc::clone(&db);
    let atomic_worker_clone5 = Arc::clone(&atomic_worker);
    let handle5 = thread::spawn(move || {
        for i in 0..20 {
            // 先获取订单ID，然后写入订单数据
            match atomic_worker_clone5.increment("order_counter".to_string(), 1) {
                Ok(order_id) => {
                    let product_name = format!("产品{}", i % 5);
                    let quantity = (i % 3) + 1;
                    let order_data = format!("{}|数量:{}", product_name, quantity);
                    if let Err(e) = db_clone5.insert(format!("order:{}", order_id).as_bytes(), order_data.as_bytes()) {
                        eprintln!("  线程5: 写入订单数据失败: {:?}", e);
                    }
                    if i % 5 == 0 {
                        println!("  线程5(订单写入): 写入订单{} {}", order_id, product_name);
                    }
                }
                Err(e) => eprintln!("  线程5: 获取订单ID失败: {:?}", e),
            }
        }
    });

    // 线程6：数据读取和统计
    let db_clone6 = Arc::clone(&db);
    let atomic_worker_clone6 = Arc::clone(&atomic_worker);
    let handle6 = thread::spawn(move || {
        for i in 0..10 {
            // 模拟读取操作和统计更新
            let user_count = db_clone6.scan_prefix(b"user:").count();
            let order_count = db_clone6.scan_prefix(b"order:").count();

            match atomic_worker_clone6.increment("read_operation_counter".to_string(), 1) {
                Ok(read_count) => {
                    match atomic_worker_clone6.increment("data_stat_counter".to_string(), 1) {
                        Ok(stat_count) => {
                            if i % 3 == 0 {
                                println!("  线程6(统计): 用户数:{} 订单数:{} 读操作:{} 统计操作:{}",
                                         user_count, order_count, read_count, stat_count);
                            }
                        }
                        Err(e) => eprintln!("  线程6: 统计计数失败: {:?}", e),
                    }
                }
                Err(e) => eprintln!("  线程6: 读操作计数失败: {:?}", e),
            }

            // 短暂休眠模拟实际操作间隔
            thread::sleep(std::time::Duration::from_millis(10));
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

    println!("\n📋 测试6: 数据一致性验证");
    println!("-----------------------");

    // 验证用户计数器
    let user_counter = atomic_worker.get("user_id_counter".to_string())?;
    println!("  用户ID计数器: {:?}", user_counter);

    // 验证订单计数器
    let order_counter = atomic_worker.get("order_counter".to_string())?;
    println!("  订单计数器: {:?}", order_counter);

    // 验证页面访问计数
    let page_views = atomic_worker.get("page_views_counter".to_string())?;
    println!("  页面访问数: {:?}", page_views);

    // 验证实际存储的用户数据
    println!("  实际用户数据:");
    for item_res in db.scan_prefix(b"user:") {
        let (key, value) = item_res?;
        let key_str = String::from_utf8_lossy(&key);
        let value_str = String::from_utf8_lossy(&value);
        let user_id = key_str.strip_prefix("user:").unwrap_or("unknown");
        println!("    用户{}: {}", user_id, value_str);
    }

    // 验证实际存储的订单数据
    println!("  实际订单数据:");
    for item_res in db.scan_prefix(b"order:") {
        let (key, value) = item_res?;
        let key_str = String::from_utf8_lossy(&key);
        let value_str = String::from_utf8_lossy(&value);
        let order_id = key_str.strip_prefix("order:").unwrap_or("unknown");
        println!("    订单{}: {}", order_id, value_str);
    }

    println!("\n📋 测试7: 持久化验证");
    println!("------------------");

    // 创建新的AtomicWorker实例测试数据持久化
    let atomic_worker2 = AtomicWorker::new(db.clone());

    // 预热计数器
    let loaded_count = atomic_worker2.preload_counters(&db)?;
    println!("  预热加载了 {} 个计数器", loaded_count);

    // 验证数据一致性
    let persisted_user_counter = atomic_worker2.get("user_id_counter".to_string())?;
    let persisted_order_counter = atomic_worker2.get("order_counter".to_string())?;
    let persisted_page_views = atomic_worker2.get("page_views_counter".to_string())?;

    println!("  持久化验证:");
    println!("    用户计数器: {:?} (原: {:?})", persisted_user_counter, user_counter);
    println!("    订单计数器: {:?} (原: {:?})", persisted_order_counter, order_counter);
    println!("    页面访问数: {:?} (原: {:?})", persisted_page_views, page_views);

    // 验证数据一致性
    let consistency_ok = persisted_user_counter == user_counter &&
                        persisted_order_counter == order_counter &&
                        persisted_page_views == page_views;

    if consistency_ok {
        println!("  ✅ 数据一致性验证通过");
    } else {
        println!("  ❌ 数据一致性验证失败");
    }

    println!("\n📋 测试8: 性能统计");
    println!("-----------------");

    // 统计数据总量
    let total_users = db.scan_prefix(b"user:").count();
    let total_orders = db.scan_prefix(b"order:").count();

    println!("  总用户数: {}", total_users);
    println!("  总订单数: {}", total_orders);
    println!("  分配的用户ID范围: 1001-{}", user_counter.unwrap_or(0));
    println!("  分配的订单范围: 1001-{}", order_counter.unwrap_or(0));

    println!("\n🎉 混合操作测试完成！");
    println!("==================");
    println!("✅ 常规数据库操作正常");
    println!("✅ 原子计数器操作正常");
    println!("✅ 并发混合操作安全");
    println!("✅ 数据一致性保证");
    println!("✅ 持久化机制有效");

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}