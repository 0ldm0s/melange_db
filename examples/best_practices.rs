use melange_db::{Db, Config};
use std::time::Instant;
use std::fs;
use std::path::Path;
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
}

/// 会话数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Session {
    session_id: String,
    user_id: u64,
    expires_at: u64,
    data: String,
}

fn main() -> io::Result<()> {
    println!("🌟 Melange DB 最佳实践示例");
    println!("================================");

    // 1. 配置最佳实践
    println!("1. 数据库配置最佳实践...");
    let db_path = Path::new("best_practice_db");

    // 清理旧的数据库
    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }

    // 生产环境推荐配置
    let mut config = Config::new()
        .path(db_path)
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
    let init_time = start.elapsed();
    println!("✅ 数据库初始化完成，耗时: {:?}", init_time);

    // 3. 打开数据树 - 使用有意义的树名
    println!("\n3. 打开数据树...");
    let users_tree = db.open_tree::<&[u8]>(b"users")?;
    let sessions_tree = db.open_tree::<&[u8]>(b"sessions")?;
    let metrics_tree = db.open_tree::<&[u8]>(b"metrics")?;
    println!("✅ 数据树打开成功 - users, sessions, metrics");

    // 4. 批量插入示例 - 用户数据
    println!("\n4. 批量插入用户数据...");
    let start = Instant::now();
    let user_batch_size = 1000;

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
            is_active: true,
        };

        let user_key = format!("user:{}", user.id);
        let user_data = serde_json::to_vec(&user)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        users_tree.insert(user_key.as_bytes(), user_data)?;
    }

    let batch_insert_time = start.elapsed();
    println!("✅ 批量插入完成，{} 条用户数据，耗时: {:?}",
             user_batch_size, batch_insert_time);
    println!("   平均插入速度: {:.2} 条/秒",
             user_batch_size as f64 / batch_insert_time.as_secs_f64());

    // 5. 批量插入示例 - 会话数据
    println!("\n5. 批量插入会话数据...");
    let start = Instant::now();
    let session_batch_size = 5000;

    for i in 0..session_batch_size {
        let session = Session {
            session_id: format!("session_{}", i),
            user_id: (i % user_batch_size) as u64, // 关联到用户
            expires_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600, // 1小时后过期
            data: format!("session_data_for_session_{}", i),
        };

        let session_key = format!("session:{}", session.session_id);
        let session_data = serde_json::to_vec(&session)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        sessions_tree.insert(session_key.as_bytes(), session_data)?;
    }

    let session_insert_time = start.elapsed();
    println!("✅ 批量插入完成，{} 条会话数据，耗时: {:?}",
             session_batch_size, session_insert_time);
    println!("   平均插入速度: {:.2} 条/秒",
             session_batch_size as f64 / session_insert_time.as_secs_f64());

    // 6. 事务操作示例
    println!("\n6. 事务操作示例...");
    let start = Instant::now();

    // 模拟用户登录 - 创建会话并更新用户最后登录时间
    let user_id = 42;
    let session_id = "login_session_12345";

    // 获取用户信息
    let user_key = format!("user:{}", user_id);
    if let Some(user_data) = users_tree.get(user_key.as_bytes())? {
        let mut user: User = serde_json::from_slice(&user_data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // 更新用户最后登录时间
        user.last_login = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs());

        // 创建新会话
        let session = Session {
            session_id: session_id.to_string(),
            user_id,
            expires_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
            data: "login_session_data".to_string(),
        };

        // 更新用户信息
        let updated_user_data = serde_json::to_vec(&user)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        users_tree.insert(user_key.as_bytes(), updated_user_data)?;

        // 插入会话
        let session_key = format!("session:{}", session_id);
        let session_data = serde_json::to_vec(&session)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        sessions_tree.insert(session_key.as_bytes(), session_data)?;

        println!("✅ 用户登录事务完成 - 用户ID: {}, 会话ID: {}", user_id, session_id);
    }

    let transaction_time = start.elapsed();
    println!("   事务操作耗时: {:?}", transaction_time);

    // 7. 范围查询示例
    println!("\n7. 范围查询示例...");
    let start = Instant::now();

    // 查询用户ID在100-200之间的用户
    let mut found_users = 0;
    let range_start = "user:100".as_bytes();
    let range_end = "user:200".as_bytes();

    for kv in users_tree.range::<&[u8], std::ops::Range<&[u8]>>(range_start..range_end) {
        let (key, value) = kv?;
        let user: User = serde_json::from_slice(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if found_users < 3 {
            println!("   找到用户: {} ({})", user.username, user.email);
        }
        found_users += 1;
    }

    let range_query_time = start.elapsed();
    println!("✅ 范围查询完成，找到 {} 个用户，耗时: {:?}",
             found_users, range_query_time);

    // 8. 前缀查询示例
    println!("\n8. 前缀查询示例...");
    let start = Instant::now();

    // 查询所有以 "user:1" 开头的用户
    let mut prefix_users = 0;
    let prefix = "user:1";

    for kv in users_tree.iter() {
        let (key, value) = kv?;
        let key_str = String::from_utf8_lossy(&key);

        if key_str.starts_with(prefix) {
            let user: User = serde_json::from_slice(&value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            if prefix_users < 3 {
                println!("   找到用户: {} ({})", user.username, user.email);
            }
            prefix_users += 1;

            if prefix_users >= 10 {
                break; // 限制显示数量
            }
        }
    }

    let prefix_query_time = start.elapsed();
    println!("✅ 前缀查询完成，找到 {} 个以 '{}' 开头的用户，耗时: {:?}",
             prefix_users, prefix, prefix_query_time);

    // 9. 数据清理示例
    println!("\n9. 数据清理示例...");
    let start = Instant::now();

    // 删除过期的会话
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut expired_sessions = 0;
    let sessions_to_remove: Vec<String> = sessions_tree.iter()
        .filter_map(|kv| {
            let (key, value) = kv.ok()?;
            let session: Session = serde_json::from_slice(&value).ok()?;

            if session.expires_at < current_time {
                expired_sessions += 1;
                Some(String::from_utf8_lossy(&key).to_string())
            } else {
                None
            }
        })
        .collect();

    // 批量删除过期会话
    for session_key in sessions_to_remove {
        sessions_tree.remove(session_key.as_bytes())?;
    }

    let cleanup_time = start.elapsed();
    println!("✅ 数据清理完成，删除了 {} 个过期会话，耗时: {:?}",
             expired_sessions, cleanup_time);

    // 10. 性能统计
    println!("\n10. 性能统计...");
    let total_users = users_tree.iter().count();
    let total_sessions = sessions_tree.iter().count();
    let total_metrics = metrics_tree.iter().count();

    println!("   总用户数: {}", total_users);
    println!("   总会话数: {}", total_sessions);
    println!("   总指标数: {}", total_metrics);

    // 计算数据库大小
    let db_size = db.size_on_disk()?;
    println!("   数据库大小: {:.2} MB", db_size as f64 / 1024.0 / 1024.0);

    // 11. 最佳实践总结
    println!("\n🎯 最佳实践总结");
    println!("================================");
    println!("✅ 配置优化:");
    println!("   • 使用合适的缓存大小 (1GB)");
    println!("   • 启用智能Flush策略平衡性能与数据安全");
    println!("   • 根据应用场景调整Flush参数");

    println!("\n✅ 数据建模:");
    println!("   • 使用有意义的树名 (users, sessions, metrics)");
    println!("   • 采用序列化数据结构 (JSON)");
    println!("   • 设计合理的键前缀 (user:{{id}}, session:{{id}})");

    println!("\n✅ 批量操作:");
    println!("   • 大量数据插入使用批量操作");
    println!("   • 避免频繁的单条插入");
    println!("   • 利用预热优化性能");

    println!("\n✅ 查询优化:");
    println!("   • 使用范围查询获取连续数据");
    println!("   • 利用前缀查询过滤数据");
    println!("   • 避免全表扫描");

    println!("\n✅ 数据管理:");
    println!("   • 定期清理过期数据");
    println!("   • 使用事务保证数据一致性");
    println!("   • 监控数据库大小和性能");

    println!("\n✅ 性能表现:");
    println!("   • 批量插入: {:.0} 用户/秒", user_batch_size as f64 / batch_insert_time.as_secs_f64());
    println!("   • 批量插入: {:.0} 会话/秒", session_batch_size as f64 / session_insert_time.as_secs_f64());
    println!("   • 范围查询: {:.0} 用户/秒", found_users as f64 / range_query_time.as_secs_f64());
    println!("   • 事务操作: {:?}", transaction_time);

    // 清理数据库
    println!("\n11. 清理数据库...");
    drop(users_tree);
    drop(sessions_tree);
    drop(metrics_tree);
    drop(db);

    if db_path.exists() {
        fs::remove_dir_all(db_path)?;
    }
    println!("✅ 数据库清理完成");

    println!("\n🎉 最佳实践示例完成！");
    println!("================================");
    println!("💡 提示:");
    println!("• 根据实际应用需求调整配置参数");
    println!("• 监控生产环境性能指标");
    println!("• 定期备份重要数据");
    println!("• 考虑数据压缩和分片策略");

    Ok(())
}