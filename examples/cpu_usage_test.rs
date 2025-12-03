//! CPU占用长期测试
//!
//! 专门测试统一入口在长期运行下的CPU占用情况

use melange_db::{Db, Config, hybrid_operations_manager::HybridOperationsManager};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    let manager = Arc::new(HybridOperationsManager::new(db.clone()));

    println!("✅ 数据库和统一路由器初始化完成");
    println!("📊 开始120秒三阶段CPU占用测试...");
    println!("🔄 阶段1: 高频操作 (0-40秒)");
    println!("🕰️  阶段2: 完全空闲 (40-80秒)");
    println!("🔄 阶段3: 低频操作 (80-120秒)");
    println!();

    // 测试参数
    let phase_duration = Duration::from_secs(40);
    let start_time = std::time::Instant::now();
    let mut operation_count = 0;
    let mut phase_ops = 0;

    // 阶段1: 高频操作 (40秒)
    println!("🚀 开始阶段1: 高频操作测试");
    while start_time.elapsed() < phase_duration {
        // 执行一些原子操作
        let counter_value = manager.increment("test_counter".to_string(), 1)?;
        operation_count += 1;
        phase_ops += 1;

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
            let ops_per_sec = phase_ops as f64 / elapsed.as_secs_f64();
            println!("⏱️  [阶段1] 已运行 {:.1}s, 完成 {} 次操作, 速率: {:.1} ops/sec",
                     elapsed.as_secs_f64(), phase_ops, ops_per_sec);
        }

        // 高频：10ms间隔
        std::thread::sleep(Duration::from_millis(10));
    }

    let phase1_ops = phase_ops;
    println!("✅ 阶段1完成: {} 次操作", phase1_ops);
    println!();

    // 阶段2: 完全空闲 (40秒)
    println!("🕰️  开始阶段2: 完全空闲测试 - 请观察CPU回落！");
    let phase2_start = Instant::now();
    while phase2_start.elapsed() < phase_duration {
        std::thread::sleep(Duration::from_secs(1));
        let elapsed = phase2_start.elapsed();
        if elapsed.as_secs() % 10 == 0 && elapsed.as_secs() > 0 {
            println!("   空闲中... 已空闲 {} 秒", elapsed.as_secs());
        }
    }
    println!("✅ 阶段2完成: 40秒完全空闲");
    println!();

    // 阶段3: 低频操作 (40秒)
    println!("🐌 开始阶段3: 低频操作测试");
    phase_ops = 0;
    let phase3_start = Instant::now();
    while phase3_start.elapsed() < phase_duration {
        // 执行原子操作
        let counter_value = manager.increment("test_counter".to_string(), 1)?;
        operation_count += 1;
        phase_ops += 1;

        // 每10次操作打印一次状态
        if phase_ops % 10 == 0 {
            let elapsed = phase3_start.elapsed();
            let ops_per_sec = phase_ops as f64 / elapsed.as_secs_f64();
            println!("⏱️  [阶段3] 已运行 {:.1}s, 完成 {} 次操作, 速率: {:.1} ops/sec",
                     elapsed.as_secs_f64(), phase_ops, ops_per_sec);
        }

        // 低频：2秒间隔
        std::thread::sleep(Duration::from_secs(2));
    }

    let phase3_ops = phase_ops;
    println!("✅ 阶段3完成: {} 次操作", phase3_ops);

    // 测试完成统计
    let total_time = start_time.elapsed();
    let final_ops_per_sec = operation_count as f64 / total_time.as_secs_f64();

    println!();
    println!("🎉 三阶段CPU测试完成！");
    println!("====================");
    println!("📈 测试统计:");
    println!("   - 总运行时间: {:.1} 秒", total_time.as_secs_f64());
    println!("   - 总操作次数: {}", operation_count);
    println!("   - 平均操作速率: {:.1} ops/sec", final_ops_per_sec);
    println!();
    println!("📊 各阶段详情:");
    println!("   - 阶段1（高频）：{} 次, ~{:.1} ops/sec", phase1_ops, phase1_ops as f64 / 40.0);
    println!("   - 阶段2（空闲）：0 次, 0 ops/sec");
    println!("   - 阶段3（低频）：{} 次, ~{:.1} ops/sec", phase3_ops, phase3_ops as f64 / 40.0);
    println!();
    println!("🔍 请检查系统监控工具中的各阶段CPU使用情况：");
    println!("   - 阶段1：CPU应该较高（持续操作）");
    println!("   - 阶段2：CPU应该显著降低（智能休眠生效）");
    println!("   - 阶段3：CPU应该适中（低频操作）");
    println!();
    println!("💡 智能休眠机制验证：");
    println!("   如果阶段2的CPU占用显著降低，说明智能休眠生效！");
    println!("   如果CPU不降低，需要进一步优化休眠策略");

    // 清理测试数据库
    std::fs::remove_dir_all("cpu_test_db").ok();

    Ok(())
}