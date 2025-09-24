use melange_db::{Db, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试：未初始化日志系统的行为");
    println!("=====================================");

    // 注意：这里没有初始化rat_logger！

    println!("创建数据库配置...");
    let config = Config::new()
        .path("test_no_logger_db")
        .cache_capacity_bytes(1024 * 1024); // 1MB 缓存

    println!("打开数据库...");
    let db: melange_db::Db<1024> = config.open()?;

    println!("执行数据库操作...");

    // 这些操作内部会调用日志宏，但由于没有初始化日志器，应该被静默忽略
    let tree = db.open_tree::<&[u8]>(b"test")?;

    println!("插入数据...");
    tree.insert(b"key1", b"value1")?;
    tree.insert(b"key2", b"value2")?;

    println!("读取数据...");
    if let Some(value) = tree.get(b"key1")? {
        println!("成功读取: {:?}", value);
    }

    println!("范围查询...");
    for kv in tree.range::<&[u8], std::ops::Range<&[u8]>>(b"key1"..b"key3") {
        let (key, value) = kv?;
        println!("找到: {:?} -> {:?}", key, value);
    }

    println!("✅ 测试完成！如果看到这条消息，说明未初始化日志不会影响程序运行");
    Ok(())
}