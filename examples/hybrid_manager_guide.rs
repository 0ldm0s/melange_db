//! 混合操作管理器使用指南
//!
//! 展示如何在保持原子操作并发安全性的同时，获得直接访问的性能

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
}

fn main() -> io::Result<()> {
    println!("🚀 Melange DB 混合操作管理器使用指南");
    println!("====================================");

    // 1. 配置数据库
    println!("1. 配置数据库...");
    let db_path = platform_utils::setup_example_db("hybrid_manager_guide");
    platform_utils::cleanup_db_directory(&db_path);

    let config = Config::new()
        .path(&db_path)
        .cache_capacity_bytes(512 * 1024 * 1024) // 512MB 缓存
        .flush_every_ms(Some(1000)); // 1秒 flush 间隔

    let db: Db<1024> = config.open()?;
    let db_arc = Arc::new(db);

    // 2. 创建混合操作管理器
    println!("2. 创建混合操作管理器...");
    let manager = HybridOperationsManager::new(db_arc.clone());

    // 3. 演示普通数据库操作（零开销）
    println!("\n3. 普通数据库操作（零开销性能）...");
    demonstrate_database_operations(&manager)?;

    // 4. 演示原子操作（并发安全）
    println!("\n4. 原子操作（并发安全）...");
    demonstrate_atomic_operations(&manager)?;

    // 5. 性能对比演示
    println!("\n5. 性能对比演示...");
    demonstrate_performance_comparison(db_arc, &manager)?;

    // 6. 实际应用场景演示
    println!("\n6. 实际应用场景演示...");
    demonstrate_real_world_scenario(&manager)?;

    // 清理
    platform_utils::cleanup_db_directory(&db_path);
    println!("\n✅ 混合管理器指南演示完成！");

    Ok(())
}

/// 演示普通数据库操作
fn demonstrate_database_operations(manager: &HybridOperationsManager) -> io::Result<()> {
    let start = Instant::now();

    // 插入用户数据
    let users = vec![
        User {
            id: 1,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            created_at: 1640995200, // 2022-01-01
            last_login: None,
            is_active: true,
        },
        User {
            id: 2,
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            created_at: 1640995300,
            last_login: Some(1640995400),
            is_active: true,
        },
        User {
            id: 3,
            username: "charlie".to_string(),
            email: "charlie@example.com".to_string(),
            created_at: 1640995500,
            last_login: None,
            is_active: false,
        },
    ];

    for user in &users {
        let key = format!("user:{}", user.id);
        let value = serde_json::to_vec(user)?;
        manager.insert(key.as_bytes(), &value)?;
        println!("  ✅ 插入用户: {} ({})", user.username, user.email);
    }

    // 查询用户
    println!("\n  🔍 查询用户数据:");
    for user_id in 1..=3 {
        let key = format!("user:{}", user_id);
        if let Some(data) = manager.get_data(key.as_bytes())? {
            let user: User = serde_json::from_slice(&*data)?;
            println!("    • {}: {} (活跃: {})", user.username, user.email, user.is_active);
        }
    }

    // 扫描操作
    println!("\n  🔍 扫描所有用户:");
    let user_results = manager.scan_prefix(b"user:")?;
    for (key, value) in user_results {
        if let Ok(key_str) = String::from_utf8(key) {
            if let Ok(user) = serde_json::from_slice::<User>(&value) {
                println!("    • {}: {}", key_str, user.username);
            }
        }
    }

    println!("  ⏱️  普通操作耗时: {:?}", start.elapsed());
    Ok(())
}

/// 演示原子操作
fn demonstrate_atomic_operations(manager: &HybridOperationsManager) -> io::Result<()> {
    let start = Instant::now();

    // 创建计数器
    println!("  📊 创建原子计数器...");

    // 页面访问计数器
    manager.reset("page_views:home".to_string(), 0)?;
    manager.reset("page_views:about".to_string(), 0)?;

    // 用户活动计数器
    manager.reset("active_users".to_string(), 0)?;
    manager.reset("total_logins".to_string(), 0)?;

    // 模拟原子操作
    println!("  🔢 执行原子操作...");

    // 模拟页面访问
    for i in 0..1000 {
        if i % 3 == 0 {
            manager.increment("page_views:home".to_string(), 1)?;
        } else {
            manager.increment("page_views:about".to_string(), 1)?;
        }

        // 每10次访问增加一个活跃用户
        if i % 10 == 0 {
            manager.increment("active_users".to_string(), 1)?;
        }

        // 每5次访问增加一次登录
        manager.increment("total_logins".to_string(), 1)?;
    }

    // 演示其他原子操作
    println!("  🧮 演示复杂原子操作...");

    // 页面访问数翻倍（促销活动）
    manager.multiply("page_views:home".to_string(), 2)?;

    // 减少50%的活跃用户（用户下线）
    manager.percentage("active_users".to_string(), 50)?;

    // 设置登录目标 - 先初始化目标值
    manager.reset("target_logins".to_string(), 0)?;
    let current_logins = manager.get("total_logins".to_string())?.unwrap_or(0);

    // 如果达到目标，设置成功标记
    if current_logins >= 1000 {
        manager.compare_and_swap("target_logins".to_string(), 0, 2000)?;
    }

    // 显示结果
    println!("\n  📈 原子计数器结果:");
    let home_views = manager.get("page_views:home".to_string())?.unwrap_or(0);
    let about_views = manager.get("page_views:about".to_string())?.unwrap_or(0);
    let active_users = manager.get("active_users".to_string())?.unwrap_or(0);
    let total_logins = manager.get("total_logins".to_string())?.unwrap_or(0);
    let target_reached = manager.get("target_logins".to_string())?.unwrap_or(0) >= 2000;

    println!("    • 首页访问量: {}", home_views);
    println!("    • 关于页面访问量: {}", about_views);
    println!("    • 活跃用户数: {}", active_users);
    println!("    • 总登录次数: {}", total_logins);
    println!("    • 目标达成: {}", if target_reached { "✅ 是" } else { "❌ 否" });

    println!("  ⏱️  原子操作耗时: {:?}", start.elapsed());
    Ok(())
}

/// 演示性能对比
fn demonstrate_performance_comparison(
    db_arc: Arc<Db<1024>>,
    manager: &HybridOperationsManager
) -> io::Result<()> {
    let test_size = 5000;

    // 测试直接访问性能
    println!("  🏃 测试直接访问性能...");
    let tree = db_arc.open_tree("performance_test")?;
    let start = Instant::now();

    for i in 0..test_size {
        let key = format!("direct_key_{}", i);
        let value = format!("direct_value_{}", i);
        tree.insert(key.as_bytes(), value.as_bytes())?;
    }

    let direct_time = start.elapsed();

    // 测试混合管理器性能
    println!("  🏃 测试混合管理器性能...");
    let start = Instant::now();

    for i in 0..test_size {
        let key = format!("hybrid_key_{}", i);
        let value = format!("hybrid_value_{}", i);
        manager.insert(key.as_bytes(), value.as_bytes())?;
    }

    let hybrid_time = start.elapsed();

    // 性能对比
    let performance_ratio = hybrid_time.as_secs_f64() / direct_time.as_secs_f64();

    println!("\n  📊 性能对比结果 ({} 条记录):", test_size);
    println!("    • 直接访问耗时: {:?}", direct_time);
    println!("    • 混合管理器耗时: {:?}", hybrid_time);
    println!("    • 性能比率: {:.2}x ({:+.1}%)",
             performance_ratio,
             (performance_ratio - 1.0) * 100.0);

    if performance_ratio < 1.1 {
        println!("    ✅ 性能表现优秀！混合管理器开销很小");
    } else {
        println!("    ⚠️  性能开销较大，可能需要优化");
    }

    Ok(())
}

/// 演示真实世界场景
fn demonstrate_real_world_scenario(manager: &HybridOperationsManager) -> io::Result<()> {
    println!("  🌍 真实世界场景：电商网站...");

    // 1. 商品管理（普通数据库操作）
    println!("\n    📦 商品管理...");
    let products = vec![
        ("prod:001", "笔记本电脑", 7999.99),
        ("prod:002", "无线鼠标", 199.99),
        ("prod:003", "机械键盘", 599.99),
        ("prod:004", "显示器", 2999.99),
    ];

    for (id, name, price) in &products {
        let product_data = format!("{}|{}", name, price);
        manager.insert(id.as_bytes(), product_data.as_bytes())?;
        println!("      ✅ 添加商品: {} (¥{})", name, price);
    }

    // 2. 库存管理（原子操作）
    println!("\n    📊 库存管理...");
    let inventory_items = vec![
        ("inventory:prod:001", 100), // 笔记本电脑
        ("inventory:prod:002", 500), // 无线鼠标
        ("inventory:prod:003", 300), // 机械键盘
        ("inventory:prod:004", 50),  // 显示器
    ];

    for (item_id, initial_stock) in inventory_items {
        manager.reset(item_id.to_string(), initial_stock)?;
        println!("      📦 初始化库存: {} = {}", item_id, initial_stock);
    }

    // 3. 销售统计（原子操作）
    println!("\n    💰 销售统计...");
    manager.reset("daily_revenue".to_string(), 0)?;
    manager.reset("daily_orders".to_string(), 0)?;
    manager.reset("daily_customers".to_string(), 0)?;

    // 模拟销售过程
    println!("      🛒 模拟销售过程...");
    let sales = vec![
        ("prod:001", 2, 7999.99), // 2台笔记本
        ("prod:002", 5, 199.99),  // 5个鼠标
        ("prod:003", 3, 599.99),  // 3个键盘
        ("prod:001", 1, 7999.99), // 1台笔记本
        ("prod:004", 2, 2999.99), // 2个显示器
    ];

    for (product_id, quantity, price) in &sales {
        // 减少库存（原子操作）
        let inventory_key = format!("inventory:{}", product_id);
        manager.decrement(inventory_key, *quantity)?;

        // 更新销售统计（原子操作）
        let revenue = *quantity as f64 * price;
        manager.increment("daily_revenue".to_string(), revenue as u64)?;
        manager.increment("daily_orders".to_string(), 1)?;
    }

    // 4. 生成销售报告
    println!("\n    📈 销售报告:");

    // 商品库存状态
    println!("      📦 当前库存:");
    for (product_id, name, _) in &products {
        let inventory_key = format!("inventory:{}", product_id);
        let current_stock = manager.get(inventory_key)?.unwrap_or(0);
        println!("        • {}: {} 件", name, current_stock);
    }

    // 销售统计
    let revenue = manager.get("daily_revenue".to_string())?.unwrap_or(0);
    let orders = manager.get("daily_orders".to_string())?.unwrap_or(0);
    println!("\n      💰 今日销售统计:");
    println!("        • 营业额: ¥{:.2}", revenue as f64 / 100.0);
    println!("        • 订单数: {}", orders);

    // 5. 热门商品查询（普通数据库操作）
    println!("\n    🔍 查询所有商品:");
    let product_results = manager.scan_prefix(b"prod:")?;
    for (key, value) in product_results {
        if let Ok(key_str) = String::from_utf8(key) {
            if let Ok(product_data) = String::from_utf8(value) {
                let parts: Vec<&str> = product_data.split('|').collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let price: f64 = parts[1].parse().unwrap_or(0.0);
                    println!("        • {}: ¥{:.2}", name, price);
                }
            }
        }
    }

    println!("  ✅ 真实场景演示完成！");
    Ok(())
}