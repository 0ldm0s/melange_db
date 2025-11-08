//! CPU占用长期测试
//!
//! 专门测试统一入口在长期运行下的CPU占用情况

use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  CPU占用长期测试");
    println!("==================");
    println!("这个测试将运行120秒来验证CPU占用修复效果");
    println!("请使用系统监控工具观察CPU使用情况");
    println!();

    // 创建数据库配置
    let config = Config::new()
        .path("cpu_test_db")
        .cache_capacity_bytes(32 * 1024 * 1024); // 32MB缓存

    // 打开数据库
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 创建统一路由器
    let manager = Arc::new(AtomicOperationsManager::new(db.clone()));

    println!("✅ 数据库和统一路由器初始化完成");
    println!("📊 开始120秒CPU占用测试...");

    // 测试参数
    let test_duration = Duration::from_secs(120);
    let start_time = std::time::Instant::now();
    let mut operation_count = 0;

    // 主测试循环
    while start_time.elapsed() < test_duration {
        // 执行一些原子操作
        let counter_value = manager.increment("test_counter".to_string(), 1)?;
        operation_count += 1;

        // 偶尔执行数据库操作
        if operation_count % 10 == 0 {
            let key = format!("key_{}", operation_count);
            let value = format!("value_{}", operation_count);
            manager.insert(key.as_bytes(), value.as_bytes())?;

            // 立即读取验证
            let _ = manager.get_data(key.as_bytes())?;
        }

        // 偶尔清理数据
        if operation_count % 50 == 0 && operation_count > 0 {
            let key_to_remove = format!("key_{}", operation_count - 40);
            let _ = manager.remove(key_to_remove.as_bytes());
        }

        // 每100次操作打印一次状态
        if operation_count % 100 == 0 {
            let elapsed = start_time.elapsed();
            let ops_per_sec = operation_count as f64 / elapsed.as_secs_f64();
            println!("⏱️  已运行 {:.1}s, 完成 {} 次操作, 速率: {:.1} ops/sec",
                     elapsed.as_secs_f64(), operation_count, ops_per_sec);
        }

        // 在操作之间短暂休眠，模拟真实使用场景
        std::thread::sleep(Duration::from_millis(10));
    }

    // 测试完成统计
    let total_time = start_time.elapsed();
    let final_ops_per_sec = operation_count as f64 / total_time.as_secs_f64();

    println!();
    println!("🎉 CPU测试完成！");
    println!("================");
    println!("📈 测试统计:");
    println!("   - 总运行时间: {:.1} 秒", total_time.as_secs_f64());
    println!("   - 总操作次数: {}", operation_count);
    println!("   - 平均操作速率: {:.1} ops/sec", final_ops_per_sec);
    println!();
    println!("🔍 请检查系统监控工具中的CPU使用情况：");
    println!("   - 修复前：CPU可能接近100%");
    println!("   - 修复后：CPU应该显著降低");
    println!();
    println!("💡 如果CPU占用仍然很高，可能需要进一步优化休眠策略");

    // 清理测试数据库
    std::fs::remove_dir_all("cpu_test_db").ok();

    Ok(())
}