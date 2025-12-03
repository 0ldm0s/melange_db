//! 统一入口remove操作测试
//!
//! 验证通过AtomicOperationsManager进行remove操作的完整功能

use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  统一入口remove操作测试");
    println!("========================");

    // 创建数据库配置
    let config = Config::new()
        .path("test_remove_db")
        .cache_capacity_bytes(32 * 1024 * 1024); // 32MB缓存

    // 打开数据库
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 创建统一路由器
    let manager = Arc::new(AtomicOperationsManager::new(db.clone()));

    println!("\n📋 测试1: 基础remove操作");
    println!("========================");

    // 插入测试数据
    let test_key = b"test:remove:key";
    let test_value = b"test_value_to_be_removed";

    manager.insert(test_key, test_value)?;
    println!("✅ 插入测试数据: {:?}", test_key);

    // 验证数据存在
    let retrieved = manager.get_data(test_key)?;
    assert!(retrieved.is_some(), "数据应该存在");
    println!("✅ 验证数据存在: {:?}", retrieved);

    // 执行remove操作
    let removed_value = manager.remove(test_key)?;
    assert!(removed_value.is_some(), "应该返回被删除的值");
    println!("✅ 成功删除数据，返回值: {:?}", removed_value);

    // 验证数据已被删除
    let should_be_none = manager.get_data(test_key)?;
    assert!(should_be_none.is_none(), "数据应该已被删除");
    println!("✅ 验证数据已删除: {:?}", should_be_none);

    println!("\n📋 测试2: 删除不存在的键");
    println!("========================");

    let non_existent_key = b"non:existent:key";
    let remove_result = manager.remove(non_existent_key)?;
    assert!(remove_result.is_none(), "删除不存在的键应该返回None");
    println!("✅ 删除不存在的键返回: {:?}", remove_result);

    println!("\n📋 测试3: 批量插入和删除");
    println!("========================");

    // 批量插入数据
    let test_prefix = b"batch:test:";
    let mut inserted_keys = Vec::new();

    for i in 1..=5 {
        let key = [test_prefix, format!("key:{}", i).as_bytes()].concat();
        let value = format!("value_{}", i).as_bytes().to_vec();

        let key_str = String::from_utf8_lossy(&key).to_string();
        let value_str = String::from_utf8_lossy(&value).to_string();
        manager.insert(&key, &value)?;
        inserted_keys.push(key);
        println!("✅ 插入数据: {} -> {}", key_str, value_str);
    }

    // 扫描验证所有数据存在
    let scan_result = manager.scan_prefix(test_prefix)?;
    assert_eq!(scan_result.len(), 5, "应该有5条数据");
    println!("✅ 扫描结果: {} 条数据", scan_result.len());

    // 逐个删除
    for (i, key) in inserted_keys.iter().enumerate() {
        let removed = manager.remove(key)?;
        assert!(removed.is_some(), "删除第{}个键应该成功", i + 1);
        println!("✅ 删除第{}个键: {:?}", i + 1, String::from_utf8_lossy(key));
    }

    // 验证所有数据已删除
    let empty_scan = manager.scan_prefix(test_prefix)?;
    assert_eq!(empty_scan.len(), 0, "扫描结果应该为空");
    println!("✅ 所有数据已删除，扫描结果: {} 条", empty_scan.len());

    println!("\n📋 测试4: 原子操作和数据库操作混合");
    println!("============================");

    // 创建计数器
    let counter_name = "test_counter".to_string();
    let counter_value = manager.increment(counter_name.clone(), 10)?;
    println!("✅ 原子操作创建计数器: {} = {}", counter_name, counter_value);

    // 创建关联数据
    let data_key = format!("counter_data:{}", counter_name);
    let data_value = format!("associated_value_{}", counter_value);
    manager.insert(data_key.as_bytes(), data_value.as_bytes())?;
    println!("✅ 创建关联数据: {} -> {}", data_key, data_value);

    // 获取并验证
    let retrieved_data = manager.get_data(data_key.as_bytes())?;
    assert!(retrieved_data.is_some(), "关联数据应该存在");
    println!("✅ 获取关联数据: {:?}", retrieved_data);

    // 删除关联数据
    let removed_data = manager.remove(data_key.as_bytes())?;
    assert!(removed_data.is_some(), "删除关联数据应该成功");
    println!("✅ 删除关联数据: {:?}", removed_data);

    // 验证计数器仍然存在（不受数据库删除影响）
    let counter_after = manager.get(counter_name.clone())?;
    assert_eq!(counter_after, Some(counter_value), "计数器应该不受影响");
    println!("✅ 验证计数器不受影响: {:?}", counter_after);

    println!("\n📋 测试5: 高压力并发remove操作");
    println!("==============================");

    let manager_clone = manager.clone();
    let mut handles = Vec::new();

    // 创建多个线程进行并发操作
    for thread_id in 0..3 {
        let manager_ref = manager_clone.clone();

        let handle = std::thread::spawn(move || -> io::Result<()> {
            for i in 0..10 {
                let key = format!("concurrent_test:{}:{}", thread_id, i);
                let value = format!("value_{}", i);

                // 插入数据
                manager_ref.insert(key.as_bytes(), value.as_bytes())?;

                // 立即删除
                let removed = manager_ref.remove(key.as_bytes())?;
                assert!(removed.is_some(), "线程{}删除第{}个数据应该成功", thread_id, i);
            }

            Ok(())
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap()?;
    }

    println!("✅ 高压力并发remove操作完成");

    // 验证没有残留数据
    let concurrent_scan = manager.scan_prefix(b"concurrent_test:")?;
    assert_eq!(concurrent_scan.len(), 0, "并发测试后应该没有残留数据");
    println!("✅ 验证无残留数据: {} 条", concurrent_scan.len());

    println!("\n🎉 所有remove操作测试通过！");
    println!("========================");
    println!("✅ 基础remove操作正常");
    println!("✅ 删除不存在键处理正确");
    println!("✅ 批量删除功能正常");
    println!("✅ 混合原子操作和数据库操作正常");
    println!("✅ 高压力并发remove操作稳定");

    println!("\n🚀 统一入口remove操作已完全集成到架构中！");

    // 清理测试数据库
    std::fs::remove_dir_all("test_remove_db").ok();

    Ok(())
}