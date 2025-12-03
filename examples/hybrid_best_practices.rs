//! Melange DB 混合管理器最佳实践
//!
//! 展示如何在实际应用中正确使用混合操作管理器

use melange_db::{Db, Config, platform_utils};
use melange_db::hybrid_operations_manager::HybridOperationsManager;
use std::sync::Arc;
use std::time::Instant;
use std::io::{self, Write};
use serde::{Serialize, Deserialize};

/// 用户数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    email: String,
    created_at: u64,
    last_login: Option<u64>,
    is_active: bool,
    login_count: u64,
}

/// 会话数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Session {
    session_id: String,
    user_id: u64,
    expires_at: u64,
    created_at: u64,
    last_activity: u64,
}

/// 应用统计
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppStats {
    total_users: u64,
    active_sessions: u64,
    daily_logins: u64,
    peak_concurrent_users: u64,
}

fn main() -> io::Result<()> {
    println!("🌟 Melange DB 混合管理器最佳实践");
    println!("==================================");

    // 1. 配置最佳实践
    println!("1. 数据库配置最佳实践...");
    let db_path = platform_utils::setup_example_db("hybrid_best_practices");
    platform_utils::cleanup_db_directory(&db_path);

    // 生产环境推荐配置
    let mut config = Config::new()
        .path(&db_path)
        .cache_capacity_bytes(1024 * 1024 * 1024) // 1GB 缓存
        .flush_every_ms(Some(1000)); // 1秒 flush 间隔

    // 启用智能 flush 策略
    config.smart_flush_config.enabled = true;
    config.smart_flush_config.base_interval_ms = 1000;
    config.smart_flush_config.min_interval_ms = 100;
    config.smart_flush_config.max_interval_ms = 5000;
    config.smart_flush_config.write_rate_threshold = 5000;
    config.smart_flush_config.accumulated_bytes_threshold = 8 * 1024 * 1024; // 8MB

    println!("✅ 配置完成 - 启用智能Flush策略和1GB缓存");

    // 2. 数据库初始化
    println!("\n2. 数据库初始化...");
    let start = Instant::now();
    let db: Db<1024> = config.open()?;
    let db_arc = Arc::new(db);
    let init_time = start.elapsed();
    println!("✅ 数据库初始化完成，耗时: {:?}", init_time);

    // 3. 创建混合操作管理器
    println!("\n3. 创建混合操作管理器...");
    let manager = HybridOperationsManager::new(db_arc.clone());
    println!("✅ 混合管理器创建完成 - 普通操作零开销，原子操作并发安全");

    // 4. 预热原子计数器
    println!("\n4. 预热原子计数器...");
    let start = Instant::now();
    let preloaded_count = manager.preload_counters()?;
    println!("✅ 预热完成，加载了 {} 个计数器，耗时: {:?}", preloaded_count, start.elapsed());

    // 5. 批量插入用户数据（高性能模式）
    println!("\n5. 批量插入用户数据...");
    let start = Instant::now();
    let user_batch_size = 10000;

    for i in 0..user_batch_size {
        let user = User {
            id: i,
            username: format!("user_{}", i),
            email: format!("user{}@example.com", i),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_login: None,
            is_active: i % 10 != 0, // 90% 活跃用户
            login_count: 0,
        };

        let user_key = format!("user:{}", user.id);
        let user_data = serde_json::to_vec(&user)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // 使用混合管理器 - 零开销直接访问
        manager.insert(user_key.as_bytes(), &user_data)?;
    }

    let batch_insert_time = start.elapsed();
    println!("✅ 批量插入完成，{} 条用户数据，耗时: {:?}",
             user_batch_size, batch_insert_time);
    println!("   平均插入速度: {:.2} 条/秒",
             user_batch_size as f64 / batch_insert_time.as_secs_f64());

    // 6. 初始化应用统计（原子操作）
    println!("\n6. 初始化应用统计...");
    let stats = AppStats {
        total_users: user_batch_size as u64,
        active_sessions: 0,
        daily_logins: 0,
        peak_concurrent_users: 0,
    };

    manager.insert(b"app_stats", &serde_json::to_vec(&stats)?)?;

    // 设置原子计数器
    manager.reset("active_sessions_count".to_string(), 0)?;
    manager.reset("current_online_users".to_string(), 0)?;
    manager.reset("total_requests".to_string(), 0)?;
    manager.reset("failed_requests".to_string(), 0)?;

    println!("✅ 应用统计初始化完成");

    // 7. 模拟用户登录（混合操作演示）
    println!("\n7. 模拟用户登录...");
    let start = Instant::now();
    let login_batch_size = 1000;

    for i in 0..login_batch_size {
        let user_id = i % user_batch_size;

        // 原子操作：增加登录计数
        manager.increment("daily_logins".to_string(), 1)?;
        manager.increment("total_requests".to_string(), 1)?;

        // 模拟登录失败（10% 概率）
        if i % 10 == 0 {
            manager.increment("failed_requests".to_string(), 1)?;
            continue;
        }

        // 创建会话（普通数据库操作）
        let session = Session {
            session_id: format!("session_{}", i),
            user_id,
            expires_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600, // 1小时后过期
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_activity: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let session_key = format!("session:{}", session.session_id);
        let session_data = serde_json::to_vec(&session)?;

        // 普通数据库操作：存储会话
        manager.insert(session_key.as_bytes(), &session_data)?;

        // 原子操作：增加活跃会话数
        manager.increment("active_sessions_count".to_string(), 1)?;

        // 更新用户登录信息（普通数据库操作）
        let user_key = format!("user:{}", user_id);
        if let Some(user_data) = manager.get_data(user_key.as_bytes())? {
            let mut user: User = serde_json::from_slice(&*user_data)?;
            user.last_login = Some(session.created_at);
            user.login_count += 1;
            user.is_active = true;

            let updated_user_data = serde_json::to_vec(&user)?;
            manager.insert(user_key.as_bytes(), &updated_user_data)?;
        }
    }

    let login_time = start.elapsed();
    println!("✅ 模拟登录完成，{} 次登录尝试，耗时: {:?}", login_batch_size, login_time);
    println!("   平均登录速度: {:.2} 次/秒",
             login_batch_size as f64 / login_time.as_secs_f64());

    // 8. 性能查询演示
    println!("\n8. 性能查询演示...");

    // 查询应用统计（原子操作）
    let daily_logins = manager.get("daily_logins".to_string())?.unwrap_or(0);
    let active_sessions = manager.get("active_sessions_count".to_string())?.unwrap_or(0);
    let total_requests = manager.get("total_requests".to_string())?.unwrap_or(0);
    let failed_requests = manager.get("failed_requests".to_string())?.unwrap_or(0);

    println!("  📊 应用统计:");
    println!("    • 今日登录次数: {}", daily_logins);
    println!("    • 活跃会话数: {}", active_sessions);
    println!("    • 总请求数: {}", total_requests);
    println!("    • 失败请求数: {}", failed_requests);
    println!("    • 成功率: {:.1}%",
             (total_requests - failed_requests) as f64 / total_requests as f64 * 100.0);

    // 查询活跃用户（普通数据库操作）
    println!("\n  👥 活跃用户查询:");
    let start = Instant::now();
    let active_user_count = manager.scan_prefix(b"user:")?
        .into_iter()
        .filter(|(_, value)| {
            if let Ok(user) = serde_json::from_slice::<User>(value) {
                user.is_active && user.last_login.is_some()
            } else {
                false
            }
        })
        .count();
    let query_time = start.elapsed();

    println!("    • 活跃用户数: {}", active_user_count);
    println!("    • 查询耗时: {:?}", query_time);

    // 9. 性能基准测试
    println!("\n9. 性能基准测试...");

    // 普通操作性能测试
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("perf_test_{}", i);
        let value = format!("value_{}", i);
        manager.insert(key.as_bytes(), value.as_bytes())?;
    }
    let normal_ops_time = start.elapsed();

    // 原子操作性能测试
    let start = Instant::now();
    for i in 0..1000 {
        manager.increment("perf_counter".to_string(), 1)?;
    }
    let atomic_ops_time = start.elapsed();

    println!("  🏃 性能测试结果:");
    println!("    • 普通操作 (1000次): {:?} ({:.2} ops/sec)",
             normal_ops_time,
             1000.0 / normal_ops_time.as_secs_f64());
    println!("    • 原子操作 (1000次): {:?} ({:.2} ops/sec)",
             atomic_ops_time,
             1000.0 / atomic_ops_time.as_secs_f64());

    // 10. 清理和优化建议
    println!("\n10. 清理和优化建议...");

    // 清理过期会话
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let start = Instant::now();
    let session_results = manager.scan_prefix(b"session:")?;
    let mut expired_sessions = 0;

    for (key, value) in session_results {
        if let Ok(session) = serde_json::from_slice::<Session>(&value) {
            if session.expires_at < now {
                manager.remove(&key)?;
                expired_sessions += 1;
            }
        }
    }

    let cleanup_time = start.elapsed();
    println!("  🧹 清理完成:");
    println!("    • 清理过期会话: {} 个", expired_sessions);
    println!("    • 清理耗时: {:?}", cleanup_time);

    // 最终性能统计
    println!("\n📈 最终性能统计:");
    println!("  • 用户数据插入: {:.2} 条/秒",
             user_batch_size as f64 / batch_insert_time.as_secs_f64());
    println!("  • 登录处理: {:.2} 次/秒",
             login_batch_size as f64 / login_time.as_secs_f64());
    println!("  • 混合管理器性能: ✅ 优秀（零开销普通操作 + 安全原子操作）");

    // 11. 最佳实践总结
    println!("\n11. 🎯 最佳实践总结:");
    println!("  ✅ 使用HybridOperationsManager获得最佳性能");
    println!("  ✅ 普通数据库操作通过直接访问实现零开销");
    println!("  ✅ 原子操作通过Worker线程保证并发安全");
    println!("  ✅ 合理配置缓存大小和智能flush策略");
    println!("  ✅ 定期清理过期数据保持性能");
    println!("  ✅ 使用批量操作提高吞吐量");

    // 清理测试数据库
    platform_utils::cleanup_db_directory(&db_path);
    println!("\n✅ 混合管理器最佳实践演示完成！");

    Ok(())
}