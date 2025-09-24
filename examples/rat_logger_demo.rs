use melange_db::{Db, Config};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化rat_logger - 由调用者配置
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .with_level(LevelFilter::Debug)
        .init()?;

    println!("🌟 Melange DB + rat_logger 集成示例");
    println!("=====================================");

    // 创建数据库配置
    let config = Config::new()
        .path("example_rat_logger_db")
        .cache_capacity_bytes(1024 * 1024) // 1MB 缓存
        .flush_every_ms(Some(1000));

    // 打开数据库
    let db: melange_db::Db<1024> = config.open()?;

    // 测试基本操作
    println!("测试基本操作...");

    // 打开或创建默认树
    let tree = db.open_tree::<&[u8]>(b"default")?;

    // 插入数据
    let key = b"test_key";
    let value = b"test_value";
    tree.insert(key, value)?;

    // 读取数据
    if let Some(retrieved) = tree.get(key)? {
        println!("成功读取数据: {:?}", retrieved);
    }

    // 删除数据
    tree.remove(key)?;

    println!("示例完成！");
    Ok(())
}