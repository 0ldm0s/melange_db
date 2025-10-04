// 简单演示：使用原子性操作实现mongoengine风格序列表

use melange_db::{Db, open};
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = open("simple_demo")?;

    // 模拟mongoengine的序列表，实现自增ID
    println!("用户ID序列:");
    for _ in 0..5 {
        let id = get_next_id(&db, "user")?;
        println!("  用户ID: {}", id);
    }

    // 模拟订单序列
    println!("\n订单ID序列:");
    for _ in 0..3 {
        let id = get_next_id(&db, "order")?;
        println!("  订单ID: {}", id);
    }

    // 并发测试验证原子性
    println!("\n并发测试验证原子性:");
    test_concurrent(std::sync::Arc::new(db.clone()))?;

    let _ = std::fs::remove_dir_all("simple_demo");
    Ok(())
}

// 核心函数：原子性获取下一个ID
fn get_next_id(db: &Db, sequence_name: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let key = format!("__seq__:{}", sequence_name);

    let result = db.update_and_fetch(&key, |current| {
        let current_val = if let Some(bytes) = current {
            u64::from_be_bytes(bytes.try_into().unwrap())
        } else {
            0
        };
        Some((current_val + 1).to_be_bytes().to_vec())
    })?;

    let bytes = result.unwrap();
    Ok(u64::from_be_bytes(bytes.as_ref().try_into().unwrap()))
}

// 并发测试验证原子性
fn test_concurrent(db: Arc<Db>) -> Result<(), Box<dyn std::error::Error>> {
    let mut handles = vec![];

    // 启动多个线程同时获取ID
    for _ in 0..3 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || -> Vec<u64> {
            let mut ids = vec![];
            for _ in 0..5 {
                if let Ok(id) = get_next_id(&*db_clone, "concurrent_test") {
                    ids.push(id);
                }
                // 添加小延迟避免过于激烈的竞争
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            ids
        });
        handles.push(handle);
    }

    // 收集所有ID
    let mut all_ids = vec![];
    for handle in handles {
        match handle.join() {
            Ok(ids) => all_ids.extend(ids),
            Err(_) => println!("线程执行失败"),
        }
    }

    // 验证是否有重复
    all_ids.sort();
    let mut has_duplicates = false;
    for i in 1..all_ids.len() {
        if all_ids[i] == all_ids[i-1] {
            has_duplicates = true;
            break;
        }
    }

    if has_duplicates {
        println!("❌ 发现重复ID，原子性失败!");
    } else {
        println!("✅ 无重复ID，原子性验证通过!");
        println!("  总共生成 {} 个唯一ID", all_ids.len());
    }

    Ok(())
}