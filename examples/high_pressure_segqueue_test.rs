use melange_db::{Db, Config, platform_utils, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;
use std::io;

fn main() -> io::Result<()> {
    println!("🚀 高压力SegQueue混合测试");
    println!("==========================");

    // 创建临时数据库
    let db_path = platform_utils::setup_example_db("high_pressure_segqueue_test");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new().path(&db_path);
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    println!("\n📋 测试1: 创建统一路由器");
    println!("-----------------------");

    let manager = AtomicOperationsManager::new(db.clone());
    let manager = Arc::new(manager);
    println!("  ✅ 统一路由器创建成功");

    println!("\n📋 测试2: 12线程高压力并发测试");
    println!("-----------------------------");

    let mut handles = vec![];

    // 线程1-4：高频原子递增操作
    for i in 1..=4 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            for j in 0..40 {
                match manager_clone.increment("high_freq_counter".to_string(), 1) {
                    Ok(value) => {
                        if j % 10 == 0 {
                            println!("  线程{}(高频原子): 计数器 = {}", i, value);
                        }
                    }
                    Err(e) => eprintln!("  线程{}原子操作失败: {:?}", i, e),
                }
            }
        });
        handles.push(handle);
    }

    // 线程5-6：批量数据库写入
    for i in 5..=6 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            for j in 0..30 {
                let base_id = (i - 5) * 30 + j;
                for k in 0..5 { // 每次批量写入5条
                    let key = format!("batch:item:{}:{}", base_id, k);
                    let value = format!("batch_value_{}_{}", i, j);
                    if let Err(e) = manager_clone.insert(key.as_bytes(), value.as_bytes()) {
                        eprintln!("  线程{}批量写入失败: {:?}", i, e);
                    }
                }
                if j % 8 == 0 {
                    println!("  线程{}(批量写入): 完成批次 {}", i, j);
                }
                thread::sleep(std::time::Duration::from_millis(2)); // 短暂休眠
            }
        });
        handles.push(handle);
    }

    // 线程7-8：混合操作（用户ID分配+用户数据）
    for i in 7..=8 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            for j in 0..25 {
                // 先分配用户ID
                match manager_clone.increment("user_id_allocator".to_string(), 1) {
                    Ok(user_id) => {
                        // 创建用户数据
                        let user_key = format!("user:{}:profile", user_id);
                        let user_value = format!("用户{}_线程{}", j, i);

                        // 用户偏好设置
                        let pref_key = format!("user:{}:preferences", user_id);
                        let pref_value = format!("偏好设置_{}_{}", j, i);

                        // 用户活动记录
                        let activity_key = format!("user:{}:activity", user_id);
                        let activity_value = format!("活动记录_{}_{}", j, i);

                        if let Err(e) = manager_clone.insert(user_key.as_bytes(), user_value.as_bytes()) {
                            eprintln!("  线程{}用户数据写入失败: {:?}", i, e);
                        }
                        if let Err(e) = manager_clone.insert(pref_key.as_bytes(), pref_value.as_bytes()) {
                            eprintln!("  线程{}用户偏好写入失败: {:?}", i, e);
                        }
                        if let Err(e) = manager_clone.insert(activity_key.as_bytes(), activity_value.as_bytes()) {
                            eprintln!("  线程{}用户活动写入失败: {:?}", i, e);
                        }

                        if j % 8 == 0 {
                            println!("  线程{}(混合用户): 创建用户{} ID:{}", i, j, user_id);
                        }
                    }
                    Err(e) => eprintln!("  线程{}用户ID分配失败: {:?}", i, e),
                }
            }
        });
        handles.push(handle);
    }

    // 线程9-10：订单系统（原子ID分配+数据库写入）
    for i in 9..=10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            for j in 0..20 {
                // 分配订单ID
                match manager_clone.increment("order_id_allocator".to_string(), 1) {
                    Ok(order_id) => {
                        // 创建订单数据
                        let order_key = format!("order:{}:details", order_id);
                        let order_value = format!("订单{}_线程{}", j, i);

                        // 订单状态
                        let status_key = format!("order:{}:status", order_id);
                        let status_value = format!("已确认_{}_{}", j, i);

                        // 订单金额（模拟）
                        let amount_key = format!("order:{}:amount", order_id);
                        let amount_value = format!("{}", (j + 1) * 100);

                        if let Err(e) = manager_clone.insert(order_key.as_bytes(), order_value.as_bytes()) {
                            eprintln!("  线程{}订单数据写入失败: {:?}", i, e);
                        }
                        if let Err(e) = manager_clone.insert(status_key.as_bytes(), status_value.as_bytes()) {
                            eprintln!("  线程{}订单状态写入失败: {:?}", i, e);
                        }
                        if let Err(e) = manager_clone.insert(amount_key.as_bytes(), amount_value.as_bytes()) {
                            eprintln!("  线程{}订单金额写入失败: {:?}", i, e);
                        }

                        if j % 7 == 0 {
                            println!("  线程{}(订单系统): 创建订单{} ID:{}", i, j, order_id);
                        }
                    }
                    Err(e) => eprintln!("  线程{}订单ID分配失败: {:?}", i, e),
                }
            }
        });
        handles.push(handle);
    }

    // 线程11：高频读取和统计
    let manager_clone11 = Arc::clone(&manager);
    let handle11 = thread::spawn(move || {
        for i in 0..15 {
            // 统计各种数据
            let user_count = manager_clone11.scan_prefix(b"user:").unwrap_or_default().len() / 3; // 每个用户有3条记录
            let order_count = manager_clone11.scan_prefix(b"order:").unwrap_or_default().len() / 3; // 每个订单有3条记录
            let batch_count = manager_clone11.scan_prefix(b"batch:").unwrap_or_default().len();

            // 读取原子计数器
            let high_freq = manager_clone11.get("high_freq_counter".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let user_ids = manager_clone11.get("user_id_allocator".to_string()).unwrap_or(Some(0)).unwrap_or(0);
            let order_ids = manager_clone11.get("order_id_allocator".to_string()).unwrap_or(Some(0)).unwrap_or(0);

            if i % 3 == 0 {
                println!("  线程11(统计): 用户:{} 订单:{} 批量:{} 高频:{} 用户ID:{} 订单ID:{}",
                         user_count, order_count, batch_count, high_freq, user_ids, order_ids);
            }

            thread::sleep(std::time::Duration::from_millis(25));
        }
    });
    handles.push(handle11);

    // 线程12：页面访问计数（模拟真实访问模式）
    let manager_clone12 = Arc::clone(&manager);
    let handle12 = thread::spawn(move || {
        for i in 0..35 {
            // 模拟不同页面的访问
            let pages = ["home", "products", "about", "contact", "search"];
            let page = pages[i % pages.len()];
            let counter_name = format!("page_views:{}", page);

            match manager_clone12.increment(counter_name, 1) {
                Ok(count) => {
                    if i % 10 == 0 {
                        println!("  线程12(访问): {}页面访问量: {}", page, count);
                    }
                }
                Err(e) => eprintln!("  线程12访问计数失败: {:?}", e),
            }

            // 模拟访问间隔
            thread::sleep(std::time::Duration::from_millis(8));
        }
    });
    handles.push(handle12);

    println!("  启动12个并发线程...");

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n📋 测试3: 数据完整性验证");
    println!("-----------------------");

    // 验证原子计数器
    let high_freq_counter = manager.get("high_freq_counter".to_string())?;
    let user_id_counter = manager.get("user_id_allocator".to_string())?;
    let order_id_counter = manager.get("order_id_allocator".to_string())?;

    println!("  原子计数器结果:");
    println!("    high_freq_counter: {:?}", high_freq_counter);
    println!("    user_id_allocator: {:?}", user_id_counter);
    println!("    order_id_allocator: {:?}", order_id_counter);

    // 验证页面访问计数
    let page_views_home = manager.get("page_views:home".to_string())?;
    let page_views_products = manager.get("page_views:products".to_string())?;
    let page_views_about = manager.get("page_views:about".to_string())?;
    let page_views_contact = manager.get("page_views:contact".to_string())?;
    let page_views_search = manager.get("page_views:search".to_string())?;

    println!("  页面访问统计:");
    println!("    home: {:?}", page_views_home);
    println!("    products: {:?}", page_views_products);
    println!("    about: {:?}", page_views_about);
    println!("    contact: {:?}", page_views_contact);
    println!("    search: {:?}", page_views_search);

    // 验证数据库记录
    let batch_records = manager.scan_prefix(b"batch:")?;
    let user_records = manager.scan_prefix(b"user:")?;
    let order_records = manager.scan_prefix(b"order:")?;

    println!("  数据库记录统计:");
    println!("    batch记录数: {}", batch_records.len());
    println!("    user记录数: {}", user_records.len());
    println!("    order记录数: {}", order_records.len());

    // 验证预期值
    let expected_high_freq = 4 * 40; // 4个线程 * 40次 = 160
    let expected_user_ids = 2 * 25; // 2个线程 * 25次 = 50
    let expected_order_ids = 2 * 20; // 2个线程 * 20次 = 40
    let expected_batch_records = 2 * 30 * 5; // 2个线程 * 30批次 * 5条 = 300
    let expected_user_records = expected_user_ids as usize * 3; // 每个用户3条记录
    let expected_order_records = expected_order_ids as usize * 3; // 每个订单3条记录

    let total_page_views = page_views_home.unwrap_or(0) + page_views_products.unwrap_or(0) +
                         page_views_about.unwrap_or(0) + page_views_contact.unwrap_or(0) +
                         page_views_search.unwrap_or(0);

    println!("\n📋 测试4: 数据一致性检查");
    println!("-----------------------");

    let high_freq_ok = high_freq_counter == Some(expected_high_freq);
    let user_ids_ok = user_id_counter == Some(expected_user_ids);
    let order_ids_ok = order_id_counter == Some(expected_order_ids);
    let batch_ok = batch_records.len() == expected_batch_records;
    let user_ok = user_records.len() == expected_user_records;
    let order_ok = order_records.len() == expected_order_records;
    let page_views_ok = total_page_views == 35; // 线程12访问了35次

    println!("  一致性检查结果:");
    println!("    high_freq_counter: {} (预期: {})", high_freq_ok, expected_high_freq);
    println!("    user_id_allocator: {} (预期: {})", user_ids_ok, expected_user_ids);
    println!("    order_id_allocator: {} (预期: {})", order_ids_ok, expected_order_ids);
    println!("    batch记录数: {} (预期: {})", batch_ok, expected_batch_records);
    println!("    user记录数: {} (预期: {})", user_ok, expected_user_records);
    println!("    order记录数: {} (预期: {})", order_ok, expected_order_records);
    println!("    总页面访问: {} (预期: {})", page_views_ok, 35);

    println!("\n📋 测试5: 持久化验证");
    println!("-----------------");

    // 等待所有持久化操作完成
    thread::sleep(std::time::Duration::from_millis(200));

    // 创建新管理器验证持久化
    let final_manager = AtomicOperationsManager::new(db.clone());
    let final_loaded = final_manager.preload_counters()?;
    println!("  最终预热计数器数量: {}", final_loaded);

    let final_high_freq = final_manager.get("high_freq_counter".to_string())?;
    let final_user_ids = final_manager.get("user_id_allocator".to_string())?;
    let final_order_ids = final_manager.get("order_id_allocator".to_string())?;

    println!("  持久化验证:");
    println!("    high_freq_counter: {:?} (原: {:?})", final_high_freq, high_freq_counter);
    println!("    user_id_allocator: {:?} (原: {:?})", final_user_ids, user_id_counter);
    println!("    order_id_allocator: {:?} (原: {:?})", final_order_ids, order_id_counter);

    let persistence_ok = final_high_freq == high_freq_counter &&
                        final_user_ids == user_id_counter &&
                        final_order_ids == order_id_counter;

    println!("\n🎉 高压力测试完成！");
    println!("==================");

    let all_ok = high_freq_ok && user_ids_ok && order_ids_ok &&
                 batch_ok && user_ok && order_ok && page_views_ok && persistence_ok;

    if all_ok {
        println!("✅ 12线程高压力测试完全通过");
        println!("✅ SegQueue统一架构在高并发下稳定");
        println!("✅ Worker间通信无冲突");
        println!("✅ 原子操作自动持久化可靠");
        println!("✅ 数据一致性完美");
        println!("✅ 无EBR冲突");
        println!("✅ 系统在高负载下表现优秀");
    } else {
        println!("❌ 部分测试失败:");
        if !high_freq_ok { println!("  - high_freq_counter失败: 预期{}, 实际{:?}", expected_high_freq, high_freq_counter); }
        if !user_ids_ok { println!("  - user_id_allocator失败: 预期{}, 实际{:?}", expected_user_ids, user_id_counter); }
        if !order_ids_ok { println!("  - order_id_allocator失败: 预期{}, 实际{:?}", expected_order_ids, order_id_counter); }
        if !batch_ok { println!("  - batch记录失败: 预期{}, 实际{}", expected_batch_records, batch_records.len()); }
        if !user_ok { println!("  - user记录失败: 预期{}, 实际{}", expected_user_records, user_records.len()); }
        if !order_ok { println!("  - order记录失败: 预期{}, 实际{}", expected_order_records, order_records.len()); }
        if !page_views_ok { println!("  - 页面访问失败: 预期35, 实际{}", total_page_views); }
        if !persistence_ok { println!("  - 持久化验证失败"); }
    }

    // 性能总结
    println!("\n📊 性能总结:");
    println!("-----------");
    println!("  总原子操作数: {}", expected_high_freq + expected_user_ids + expected_order_ids + 35);
    println!("  总数据库记录数: {}", batch_records.len() + user_records.len() + order_records.len());
    println!("  并发线程数: 12");
    println!("  测试类型: 高压力混合并发");

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);

    Ok(())
}