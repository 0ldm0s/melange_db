use melange_db::*;
use std::time::Instant;

#[test]
fn test_basic_operations() {
    let config = Config::new()
        .path("basic_integration_test_db");

    // 确保测试目录干净
    if std::path::Path::new("basic_integration_test_db").exists() {
        std::fs::remove_dir_all("basic_integration_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("basic_ops").unwrap();

    // 测试插入
    tree.insert(b"key1", b"value1").unwrap();
    tree.insert(b"key2", b"value2").unwrap();
    tree.insert(b"key3", b"value3").unwrap();

    // 测试读取
    assert_eq!(tree.get(b"key1").unwrap(), Some(InlineArray::from(b"value1".as_slice())));
    assert_eq!(tree.get(b"key2").unwrap(), Some(InlineArray::from(b"value2".as_slice())));
    assert_eq!(tree.get(b"key3").unwrap(), Some(InlineArray::from(b"value3".as_slice())));
    assert_eq!(tree.get(b"nonexistent").unwrap(), None);

    // 测试删除
    tree.remove(b"key2").unwrap();
    assert_eq!(tree.get(b"key2").unwrap(), None);

    // 测试迭代
    let keys: Vec<InlineArray> = tree.iter().map(|r| r.unwrap().0).collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&InlineArray::from(b"key1".as_slice())));
    assert!(keys.contains(&InlineArray::from(b"key3".as_slice())));

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("basic_integration_test_db").unwrap();
}

#[test]
fn test_concurrent_operations() {
    let config = Config::new()
        .path("concurrent_integration_test_db");

    if std::path::Path::new("concurrent_integration_test_db").exists() {
        std::fs::remove_dir_all("concurrent_integration_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("concurrent_ops").unwrap();

    // 简单的并发测试 - 在单线程中模拟并发操作
    for thread_id in 0..4 {
        for i in 0..100 {
            let key = format!("thread_{}_key_{}", thread_id, i);
            let value = format!("value_{}", i);
            tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
        }
    }

    // 验证结果
    let count = tree.len().unwrap();
    assert_eq!(count, 400);

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("concurrent_integration_test_db").unwrap();
}

#[test]
fn test_flush_scheduler() {
    let config = Config::new()
        .path("flush_integration_test_db")
        .flush_every_ms(Some(50));

    if std::path::Path::new("flush_integration_test_db").exists() {
        std::fs::remove_dir_all("flush_integration_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("flush_test").unwrap();

    // 插入一些数据
    for i in 0..100 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    // 执行flush
    tree.flush().unwrap();

    // 等待一小段时间确保flush完成
    std::thread::sleep(std::time::Duration::from_millis(100));

    // 验证数据还在
    assert_eq!(tree.len().unwrap(), 100);

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("flush_integration_test_db").unwrap();
}

#[test]
fn test_incremental_serialization() {
    let config = Config::new()
        .path("incremental_integration_test_db")
        .incremental_serialization_threshold(10);

    if std::path::Path::new("incremental_integration_test_db").exists() {
        std::fs::remove_dir_all("incremental_integration_test_db").unwrap();
    }

    let db = config.open::<1024>().unwrap();
    let tree = db.open_tree("incremental_test").unwrap();

    // 插入初始数据
    for i in 0..20 {
        let key = format!("key_{}", i);
        let value = format!("initial_value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    // 更新部分数据
    for i in 0..5 {
        let key = format!("key_{}", i);
        let new_value = format!("updated_value_{}", i);
        tree.insert(key.as_bytes(), new_value.as_bytes()).unwrap();
    }

    // 验证更新后的数据
    for i in 0..5 {
        let key = format!("key_{}", i);
        let expected_value = format!("updated_value_{}", i);
        assert_eq!(tree.get(key.as_bytes()).unwrap(), Some(InlineArray::from(expected_value.as_bytes())));
    }

    // 验证未更新的数据
    for i in 5..20 {
        let key = format!("key_{}", i);
        let expected_value = format!("initial_value_{}", i);
        assert_eq!(tree.get(key.as_bytes()).unwrap(), Some(InlineArray::from(expected_value.as_bytes())));
    }

    // 清理
    drop(tree);
    drop(db);
    std::fs::remove_dir_all("incremental_integration_test_db").unwrap();
}