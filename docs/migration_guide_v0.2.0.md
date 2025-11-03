# Melange DB v0.2.0 迁移指南

## 概述

Melange DB v0.2.0 是一个**破坏性性能升级**版本，引入了全新的原子操作统一架构，完全解决了高并发场景下的 EBR (Epoch-Based Reclamation) RefCell 冲突问题。

虽然这是一个破坏性升级，但我们努力使迁移过程尽可能简单。本指南将帮助您从旧版本安全升级到 v0.2.0。

## 🚨 主要变更

### 解决的问题
- ✅ **完全消除 EBR RefCell 冲突**: 多线程高并发操作不再出现 `RefCell already borrowed` panic
- ✅ **提升并发性能**: 通过 Worker 间通信大幅提升并发性能
- ✅ **数据一致性保证**: 确保高并发下的数据完整性

### API 变更
- 🔄 **AtomicOperationsManager**: 新的统一路由器设计
- 🔄 **AtomicWorker**: 重构为完全独立的原子操作组件
- 🆕 **DatabaseWorker**: 新增专用数据库操作 Worker

## 迁移步骤

### 步骤 1: 更新依赖版本

**Cargo.toml**:
```toml
[dependencies]
# 旧版本
melange_db = "0.1.5"

# 新版本
melange_db = "0.2.0"
```

### 步骤 2: 更新代码结构

#### 旧版本代码 (v0.1.5 及以下)
```rust
// ❌ 这种写法会导致 EBR 冲突
use melange_db::{Db, Config};
use std::sync::Arc;
use std::thread;

fn main() -> anyhow::Result<()> {
    let config = Config::new().path("my_db");
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 多线程直接操作数据库 - 会产生 EBR 冲突！
    let mut handles = vec![];
    for i in 0..4 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            // 这些操作在高并发下会导致 RefCell panic
            let tree = db_clone.open_tree("counters").unwrap();
            tree.increment(&format!("counter_{}", i)).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}
```

#### 新版本代码 (v0.2.0+)
```rust
// ✅ 推荐写法 - 无 EBR 冲突
use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;

fn main() -> anyhow::Result<()> {
    let config = Config::new().path("my_db");
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 创建统一路由器
    let manager = Arc::new(AtomicOperationsManager::new(db));

    // 多线程通过统一路由器操作 - 完全安全！
    let mut handles = vec![];
    for i in 0..4 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            // 原子操作 - 自动持久化
            let counter = manager_clone.increment(format!("counter_{}", i), 1).unwrap();
            println!("Thread {} counter: {}", i, counter);

            // 数据库操作 - 也是安全的
            let key = format!("data:{}", i);
            let value = format!("value_from_thread_{}", i);
            manager_clone.insert(key.as_bytes(), value.as_bytes()).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}
```

### 步骤 3: 测试迁移

运行以下测试验证迁移是否成功：

```bash
# 基础统一架构测试
cargo run --example segqueue_unified_test

# 高压力并发测试 (12线程)
cargo run --example high_pressure_segqueue_test

# 原子操作测试
cargo run --example atomic_worker_test
```

## 常见迁移场景

### 场景 1: 原子计数器

**旧代码**:
```rust
// ❌ 旧方式 - 可能有 EBR 冲突
let tree = db.open_tree("counters")?;
let new_value = tree.increment("user_counter")?;
```

**新代码**:
```rust
// ✅ 新方式 - 完全安全
let new_value = manager.increment("user_counter".to_string(), 1)?;
```

### 场景 2: 用户ID分配

**旧代码**:
```rust
// ❌ 旧方式
let user_id = tree.increment("user_id_allocator")?;
let user_key = format!("user:{}", user_id);
tree.insert(user_key.as_bytes(), user_data.as_bytes())?;
```

**新代码**:
```rust
// ✅ 新方式
let user_id = manager.increment("user_id_allocator".to_string(), 1)?;
let user_key = format!("user:{}", user_id);
manager.insert(user_key.as_bytes(), user_data.as_bytes())?;
```

### 场景 3: 批量操作

**旧代码**:
```rust
// ❌ 旧方式 - 高并发下可能崩溃
for i in 0..1000 {
    let tree = db.open_tree("batch_data")?;
    tree.insert(&format!("key_{}", i), &format!("value_{}", i))?;
}
```

**新代码**:
```rust
// ✅ 新方式 - 完全安全
for i in 0..1000 {
    let key = format!("key_{}", i);
    let value = format!("value_{}", i);
    manager.insert(key.as_bytes(), value.as_bytes())?;
}
```

## 性能对比

### 并发性能

| 指标 | v0.1.5 | v0.2.0 | 改进 |
|------|--------|--------|------|
| 并发线程支持 | 2-4 线程 | 无限制 | ∞ |
| EBR 冲突 | 频繁发生 | 零冲突 | 100% |
| 数据一致性 | 可能损坏 | 完全保证 | 100% |

### 测试结果

**12线程高压力测试**:
- ✅ 285次原子操作 (160 + 50 + 40 + 35次页面访问)
- ✅ 570条数据库记录 (300 + 150 + 120条)
- ✅ 零EBR冲突
- ✅ 100%数据一致性

## 兼容性说明

### 数据兼容性
- ✅ **完全向后兼容**: v0.1.5 创建的数据库文件可以在 v0.2.0 中正常读取
- ✅ **无需数据迁移**: 现有数据无需任何转换操作

### API兼容性
- ❌ **破坏性变更**: 原子操作API需要重写
- ✅ **基础API不变**: 普通的数据库读写API保持不变
- ❌ **并发模式变更**: 多线程并发访问模式需要更新

## 故障排除

### 问题 1: 编译错误

**错误**: `cannot find function AtomicOperationsManager`

**解决**: 确保版本正确更新：
```bash
cargo clean
cargo update
```

### 问题 2: 运行时错误

**错误**: 找不到原子计数器数据

**解决**: 使用预热功能加载旧数据：
```rust
// 预热现有的原子计数器
let loaded_count = manager.preload_counters()?;
println!("预加载了 {} 个计数器", loaded_count);
```

### 问题 3: 性能问题

**现象**: 升级后性能变慢

**解决**: 检查是否正确使用统一路由器：
```rust
// ✅ 正确 - 所有操作通过 manager
let value = manager.increment("counter".to_string(), 1)?;
manager.insert(key, value)?;

// ❌ 错误 - 混用新旧API
let db = manager.db_worker().db(); // 不要这样做！
```

## 回滚方案

如果升级过程中遇到问题，可以临时回滚到旧版本：

```toml
# 临时回滚
melange_db = "0.1.5"
```

**注意**: 回滚前请备份您的数据库文件！

## 获取帮助

如果在迁移过程中遇到问题：

1. **查看示例代码**: `examples/` 目录下的完整示例
2. **运行测试**: 使用提供的测试用例验证功能
3. **检查日志**: 启用详细日志查看具体错误信息

## 总结

v0.2.0 的迁移虽然需要一些代码修改，但带来的好处是巨大的：

- 🚀 **零并发冲突**: 彻底解决EBR问题
- 📈 **无限并发性**: 支持任意数量的并发线程
- 🔒 **数据一致性**: 完全保证高并发下的数据完整性
- ⚡ **性能提升**: 整体并发性能显著改善

按照本指南的步骤，您可以安全、顺利地完成升级。

---

**升级后强烈建议运行完整的测试套件**:
```bash
cargo test
cargo run --example segqueue_unified_test
cargo run --example high_pressure_segqueue_test
```

祝您使用愉快！🎉