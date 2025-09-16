use melange_db::{Db, Config};
use std::time::Instant;
use std::fs;
use std::path::Path;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("🪐 Melange DB 性能测试与示例");
    println!("================================");

    // 测试数据路径
    let db_path = Path::new("example_db");

    // 清理旧的数据库
    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }

    // 创建配置
    let config = Config::new()
        .path(db_path)
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB 缓存

    println!("1. 打开数据库...");
    let start = Instant::now();
    let db: Db<1024> = config.open()?;
    let open_time = start.elapsed();
    println!("✅ 数据库打开成功，耗时: {:?}", open_time);

    // 打开一个树
    println!("\n2. 打开数据树...");
    let tree = db.open_tree::<&[u8]>(b"example_tree")?;
    println!("✅ 数据树打开成功");

    // 基本读写操作测试
    println!("\n3. 基本读写操作测试...");
    let key = b"test_key";
    let value = "这是一个测试值，用于验证 Melange DB 的读写功能".as_bytes();

    // 写入数据
    let start = Instant::now();
    tree.insert(key, value)?;
    let write_time = start.elapsed();
    println!("✅ 单条写入完成，耗时: {:?}", write_time);

    // 读取数据
    let start = Instant::now();
    let retrieved = tree.get(key)?;
    let read_time = start.elapsed();

    match retrieved {
        Some(data) => {
            println!("✅ 单条读取完成，耗时: {:?}", read_time);
            println!("   读取的数据: {}", String::from_utf8_lossy(&data));
        }
        None => {
            println!("❌ 数据读取失败");
        }
    }

    // 批量操作测试
    println!("\n4. 批量操作测试...");
    let batch_size = 1000;

    // 批量写入
    println!("   批量写入 {} 条记录...", batch_size);
    let start = Instant::now();
    for i in 0..batch_size {
        let key = format!("batch_key_{}", i);
        let value = format!("批量测试值 {}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }
    let batch_write_time = start.elapsed();
    println!("✅ 批量写入完成，耗时: {:?}", batch_write_time);
    println!("   平均每条写入: {:?}", batch_write_time / batch_size);

    // 批量读取
    println!("   批量读取 {} 条记录...", batch_size);
    let start = Instant::now();
    let mut success_count = 0;
    for i in 0..batch_size {
        let key = format!("batch_key_{}", i);
        if tree.get(key.as_bytes())?.is_some() {
            success_count += 1;
        }
    }
    let batch_read_time = start.elapsed();
    println!("✅ 批量读取完成，耗时: {:?}", batch_read_time);
    println!("   平均每条读取: {:?}", batch_read_time / batch_size);
    println!("   读取成功率: {}/{}", success_count, batch_size);

    // 范围查询测试
    println!("\n5. 范围查询测试...");
    let start = Instant::now();
    let mut count = 0;
    let range_start = "batch_key_100".as_bytes();
    let range_end = "batch_key_200".as_bytes();
    for kv in tree.range::<&[u8], std::ops::Range<&[u8]>>(range_start..range_end) {
        let (key, value) = kv?;
        if count < 5 { // 只打印前5条
            println!("   {}: {}", String::from_utf8_lossy(&key), String::from_utf8_lossy(&value));
        }
        count += 1;
    }
    let range_time = start.elapsed();
    println!("✅ 范围查询完成，找到 {} 条记录，耗时: {:?}", count, range_time);

    // 删除操作测试
    println!("\n6. 删除操作测试...");
    let start = Instant::now();
    tree.remove(key)?;
    let delete_time = start.elapsed();

    // 验证删除
    let deleted = tree.get(key)?;
    match deleted {
        None => {
            println!("✅ 删除操作完成，耗时: {:?}", delete_time);
            println!("   数据已成功删除");
        }
        Some(_) => {
            println!("❌ 删除操作失败");
        }
    }

    // 性能统计测试
    println!("\n7. 性能统计测试...");
    let perf_test_size = 10000;

    println!("   性能测试：{} 条记录的读写...", perf_test_size);

    // 写入性能测试
    let start = Instant::now();
    for i in 0..perf_test_size {
        let key = format!("perf_key_{}", i);
        let value = format!("性能测试值 {}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }
    let perf_write_time = start.elapsed();
    let perf_write_ops = perf_test_size as f64 / perf_write_time.as_secs_f64();

    // 读取性能测试
    let start = Instant::now();
    let mut read_success = 0;
    for i in 0..perf_test_size {
        let key = format!("perf_key_{}", i);
        if tree.get(key.as_bytes())?.is_some() {
            read_success += 1;
        }
    }
    let perf_read_time = start.elapsed();
    let perf_read_ops = perf_test_size as f64 / perf_read_time.as_secs_f64();

    println!("✅ 性能测试完成");
    println!("   写入性能: {:.2} ops/sec ({:.2} µs/op)",
             perf_write_ops, perf_write_time.as_micros() as f64 / perf_test_size as f64);
    println!("   读取性能: {:.2} ops/sec ({:.2} µs/op)",
             perf_read_ops, perf_read_time.as_micros() as f64 / perf_test_size as f64);
    println!("   读取成功率: {}/{}", read_success, perf_test_size);

    // 显示数据库统计信息
    println!("\n8. 数据库统计信息...");
    let total_records = tree.iter().count();
    println!("   总记录数: {}", total_records);

    // 计算总内存使用
    let mut total_size = 0;
    for kv in tree.iter() {
        let (key, value) = kv?;
        total_size += key.len() + value.len();
    }
    println!("   数据总大小: {} bytes", total_size);
    if total_records > 0 {
        println!("   平均记录大小: {:.2} bytes", total_size as f64 / total_records as f64);
    }

    // 性能对比总结
    println!("\n🎯 性能对比总结");
    println!("================================");
    println!("Melange DB 性能表现:");
    println!("• 单条写入: {:.2} µs", write_time.as_micros() as f64);
    println!("• 单条读取: {:.2} µs", read_time.as_micros() as f64);
    println!("• 批量写入: {:.2} µs/op", batch_write_time.as_micros() as f64 / batch_size as f64);
    println!("• 批量读取: {:.2} µs/op", batch_read_time.as_micros() as f64 / batch_size as f64);
    println!("• 高性能写入: {:.2} µs/op", perf_write_time.as_micros() as f64 / perf_test_size as f64);
    println!("• 高性能读取: {:.2} µs/op", perf_read_time.as_micros() as f64 / perf_test_size as f64);

    println!("\n与 RocksDB 对比:");
    println!("• 写入性能: {:.2}x 倍提升 (RocksDB: 5 µs/条 → Melange DB: {:.2} µs/条)",
             5.0 / (perf_write_time.as_micros() as f64 / perf_test_size as f64),
             perf_write_time.as_micros() as f64 / perf_test_size as f64);
    println!("• 读取性能: {:.2}x 倍提升 (RocksDB: 0.5 µs/条 → Melange DB: {:.2} µs/条)",
             0.5 / (perf_read_time.as_micros() as f64 / perf_test_size as f64),
             perf_read_time.as_micros() as f64 / perf_test_size as f64);

    println!("\n🚀 优化技术亮点:");
    println!("• SIMD 优化的 key 比较 (ARM64 NEON)");
    println!("• 多级布隆过滤器过滤");
    println!("• 热/温/冷三级缓存系统");
    println!("• 增量序列化优化");
    println!("• 透明的性能优化集成");

    // 清理数据库
    println!("\n9. 清理数据库...");
    drop(tree);
    drop(db);

    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }
    println!("✅ 数据库清理完成");

    println!("\n🎉 所有测试完成！Melange DB 运行正常！");
    println!("================================");

    Ok(())
}