//! 统一入口完整API综合测试
//!
//! 验证AtomicOperationsManager支持的所有数据库操作

use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 统一入口完整API综合测试");
    println!("============================");

    // 创建数据库配置
    let config = Config::new()
        .path("comprehensive_test_db")
        .cache_capacity_bytes(32 * 1024 * 1024); // 32MB缓存

    // 打开数据库
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 创建统一路由器
    let manager = Arc::new(AtomicOperationsManager::new(db.clone()));

    println!("\n📋 测试1: 基础CRUD操作");
    println!("======================");

    // 插入测试数据
    manager.insert(b"key1", b"value1")?;
    manager.insert(b"key2", b"value2")?;
    manager.insert(b"key3", b"value3")?;
    println!("✅ 插入3条测试数据");

    // 获取数据
    let value1 = manager.get_data(b"key1")?;
    assert!(value1.is_some(), "key1应该存在");
    println!("✅ 获取数据: key1 -> {:?}", value1);

    // 更新数据（通过插入覆盖）
    manager.insert(b"key1", b"updated_value1")?;
    let updated_value1 = manager.get_data(b"key1")?;
    assert_eq!(updated_value1, Some(b"updated_value1".to_vec().into()));
    println!("✅ 更新数据: key1 -> {:?}", updated_value1);

    // 删除数据
    let removed = manager.remove(b"key2")?;
    assert!(removed.is_some(), "删除key2应该返回值");
    println!("✅ 删除数据: key2 -> {:?}", removed);

    // 验证删除
    let should_be_none = manager.get_data(b"key2")?;
    assert!(should_be_none.is_none(), "key2应该已被删除");
    println!("✅ 验证删除成功");

    println!("\n📋 测试2: contains_key 操作");
    println!("=========================");

    // 检查存在的键
    let key1_exists = manager.contains_key(b"key1")?;
    assert!(key1_exists, "key1应该存在");
    println!("✅ key1 存在: {}", key1_exists);

    // 检查不存在的键
    let key2_exists = manager.contains_key(b"key2")?;
    assert!(!key2_exists, "key2应该不存在");
    println!("✅ key2 存在: {}", key2_exists);

    // 检查不存在的键
    let non_existent = manager.contains_key(b"non_existent_key")?;
    assert!(!non_existent, "不存在的键应该返回false");
    println!("✅ 不存在的键存在: {}", non_existent);

    println!("\n📋 测试3: len 和 is_empty 操作");
    println!("=============================");

    // 获取当前长度
    let current_len = manager.len()?;
    println!("✅ 当前键值对数量: {}", current_len);
    assert_eq!(current_len, 2, "应该有2个键值对（key1和key3）");

    // 检查是否为空
    let is_empty = manager.is_empty()?;
    println!("✅ 数据库是否为空: {}", is_empty);
    assert!(!is_empty, "数据库不应该为空");

    // 清空数据库
    manager.clear()?;
    println!("✅ 清空数据库");

    // 再次检查
    let after_clear_len = manager.len()?;
    let after_clear_empty = manager.is_empty()?;
    println!("✅ 清空后数量: {}, 是否为空: {}", after_clear_len, after_clear_empty);
    assert_eq!(after_clear_len, 0, "清空后数量应该为0");
    assert!(after_clear_empty, "清空后应该为空");

    println!("\n📋 测试4: first 和 last 操作");
    println!("==========================");

    // 重新插入一些测试数据
    manager.insert(b"apple", b"red")?;
    manager.insert(b"banana", b"yellow")?;
    manager.insert(b"cherry", b"red")?;
    println!("✅ 插入3个水果数据");

    // 获取第一个键值对
    let first = manager.first()?;
    assert!(first.is_some(), "应该有第一个键值对");
    let (first_key, first_value) = first.unwrap();
    println!("✅ 第一个键值对: {:?} -> {:?}",
             String::from_utf8_lossy(&first_key),
             String::from_utf8_lossy(&first_value));

    // 获取最后一个键值对
    let last = manager.last()?;
    assert!(last.is_some(), "应该有最后一个键值对");
    let (last_key, last_value) = last.unwrap();
    println!("✅ 最后一个键值对: {:?} -> {:?}",
             String::from_utf8_lossy(&last_key),
             String::from_utf8_lossy(&last_value));

    println!("\n📋 测试5: 空数据库的边界操作");
    println!("===========================");

    // 清空数据库
    manager.clear()?;
    println!("✅ 清空数据库进行边界测试");

    // 空数据库的边界操作
    let empty_first = manager.first()?;
    let empty_last = manager.last()?;
    let empty_len = manager.len()?;
    let empty_is_empty = manager.is_empty()?;

    println!("✅ 空数据库操作:");
    println!("   - first(): {:?}", empty_first);
    println!("   - last(): {:?}", empty_last);
    println!("   - len(): {}", empty_len);
    println!("   - is_empty(): {}", empty_is_empty);

    assert!(empty_first.is_none(), "空数据库的第一个应该为None");
    assert!(empty_last.is_none(), "空数据库的最后一个应该为None");
    assert_eq!(empty_len, 0, "空数据库的长度应该为0");
    assert!(empty_is_empty, "空数据库应该为空");

    println!("\n📋 测试6: scan_prefix 与新操作结合");
    println!("==================================");

    // 插入一些带前缀的数据
    manager.insert(b"user:1001", b"Alice")?;
    manager.insert(b"user:1002", b"Bob")?;
    manager.insert(b"user:1003", b"Charlie")?;
    manager.insert(b"product:1001", b"Laptop")?;
    manager.insert(b"product:1002", b"Mouse")?;
    println!("✅ 插入用户和产品数据");

    // 扫描用户前缀
    let users = manager.scan_prefix(b"user:")?;
    println!("✅ 扫描用户数据: {} 条", users.len());
    for (key, value) in &users {
        println!("   - {:?} -> {:?}",
                 String::from_utf8_lossy(key),
                 String::from_utf8_lossy(value));
    }

    // 检查用户数据存在性
    let has_user1001 = manager.contains_key(b"user:1001")?;
    let has_user9999 = manager.contains_key(b"user:9999")?;
    println!("✅ 用户1001存在: {}, 用户9999存在: {}", has_user1001, has_user9999);

    // 删除一个用户
    let removed_user = manager.remove(b"user:1002")?;
    println!("✅ 删除用户1002: {:?}", removed_user);

    // 再次扫描验证
    let users_after = manager.scan_prefix(b"user:")?;
    println!("✅ 删除后用户数据: {} 条", users_after.len());

    println!("\n📋 测试7: 与原子操作混合使用");
    println!("============================");

    // 创建原子计数器
    let user_counter = manager.increment("user_count".to_string(), 0)?;
    println!("✅ 创建用户计数器: {}", user_counter);

    // 插入用户数据
    let user_id = manager.increment("user_count".to_string(), 1)?;
    let user_key = format!("user:{}", user_id);
    manager.insert(user_key.as_bytes(), b"New User")?;
    println!("✅ 插入新用户: {} -> New User", user_key);

    // 检查用户数据
    let user_exists = manager.contains_key(user_key.as_bytes())?;
    let current_count = manager.get("user_count".to_string())?;
    let total_records = manager.len()?;
    println!("✅ 用户存在: {}, 计数器: {}, 总记录数: {}",
             user_exists, current_count.unwrap_or(0), total_records);

    // 删除用户数据但保留计数器
    let removed_user_data = manager.remove(user_key.as_bytes())?;
    let user_data_exists_after = manager.contains_key(user_key.as_bytes())?;
    let counter_after_remove = manager.get("user_count".to_string())?;
    println!("✅ 删除用户数据: {:?}, 用户数据存在: {}, 计数器仍存在: {}",
             removed_user_data, user_data_exists_after, counter_after_remove.unwrap_or(0));

    println!("\n📋 测试8: 高压力混合操作");
    println!("========================");

    let manager_clone = manager.clone();
    let mut handles = Vec::new();

    // 创建多个线程进行混合操作
    for thread_id in 0..3 {
        let manager_ref = manager_clone.clone();

        let handle = std::thread::spawn(move || -> std::io::Result<()> {
            for i in 0..20 {
                let key = format!("mixed:{}:{}", thread_id, i);
                let value = format!("value_{}", i);

                // 插入数据
                manager_ref.insert(key.as_bytes(), value.as_bytes())?;

                // 检查存在性
                let exists = manager_ref.contains_key(key.as_bytes())?;
                assert!(exists, "数据应该存在");

                // 立即删除
                let removed = manager_ref.remove(key.as_bytes())?;
                assert!(removed.is_some(), "删除应该成功");

                // 检查已删除
                let not_exists = manager_ref.contains_key(key.as_bytes())?;
                assert!(!not_exists, "删除后应该不存在");
            }

            Ok(())
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap()?;
    }

    println!("✅ 高压力混合操作完成");

    // 验证最终状态
    let final_len = manager.len()?;
    let final_is_empty = manager.is_empty()?;
    let final_users = manager.scan_prefix(b"user:")?;
    let final_products = manager.scan_prefix(b"product:")?;

    println!("✅ 最终状态:");
    println!("   - 总记录数: {}", final_len);
    println!("   - 是否为空: {}", final_is_empty);
    println!("   - 用户记录: {} 条", final_users.len());
    println!("   - 产品记录: {} 条", final_products.len());

    println!("\n🎉 所有统一入口API测试通过！");
    println!("==============================");
    println!("✅ CRUD操作完整");
    println!("✅ contains_key操作正常");
    println!("✅ len和is_empty操作正常");
    println!("✅ first和last操作正常");
    println!("✅ clear操作正常");
    println!("✅ 边界情况处理正确");
    println!("✅ 与原子操作混合使用正常");
    println!("✅ 高压力并发操作稳定");

    println!("\n🚀 统一入口API现已完整支持所有常用数据库操作！");

    // 清理测试数据库
    std::fs::remove_dir_all("comprehensive_test_db").ok();

    Ok(())
}