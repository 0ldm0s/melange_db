# Melange DB 🪐

> "The spice must flow..." - 基于沙丘灵感的下一代嵌入式数据库

## 项目理念

Melange DB 是一个受沙丘小说启发的嵌入式数据库项目，专注于高性能和可扩展性。就像香料Melange能够扩展意识一样，我们的数据库旨在扩展数据处理的边界。

## 核心特性

### 🚀 高性能架构
- **异步优先设计**: 基于tokio的完全异步IO
- **零拷贝序列化**: 极致的序列化性能优化
- **智能缓存**: 自适应缓存替换策略

### 🔒 并发安全
- **无锁数据结构**: 最大限度减少锁竞争
- **线程局部存储**: 优化多线程性能
- **原子性保证**: 完全ACID兼容

### 📦 模块化设计
- **可插拔存储引擎**: 支持多种底层存储格式
- **可配置压缩**: 多种压缩算法支持
- **扩展性架构**: 易于添加新功能和优化

## 快速开始

```rust
use melange_db::{Db, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 打开数据库
    let db = Db::open(Config::default()).await?;

    // 写入数据
    db.put(b"key", b"value").await?;

    // 读取数据
    if let Some(value) = db.get(b"key").await? {
        println!("Found value: {:?}", value);
    }

    Ok(())
}
```

## 性能目标

- **写入吞吐量**: 1M+ ops/sec
- **读取延迟**: <100μs P99
- **内存效率**: 比sled减少50%内存使用
- **并发支持**: 1000+ 并发连接

## 架构设计

### 核心组件

1. **Spice Kernel**: 核心存储引擎
2. **Fremen Cache**: 智能缓存系统
3. **Bene Gesserit Index**: 高级索引结构
4. **Guild Navigator**: 查询优化器
5. **Sardaukar Storage**: 持久化层

## 开发路线

### Phase 1: 基础架构 (沙丘: 第1部)
- [ ] 异步IO子系统
- [ ] 基本KV存储接口
- [ ] 内存管理框架

### Phase 2: 性能优化 (沙丘: 第2部)
- [ ] 增量序列化
- [ ] 无锁数据结构
- [ ] 智能缓存策略

### Phase 3: 高级特性 (沙丘: 第3部)
- [ ] 分布式支持
- [ ] 事务处理
- [ ] 流式处理


## 灵感来源

本项目受到 Frank Herbert 的《沙丘》小说系列启发，旨在创造像香料Melange一样能够扩展数据处理能力的数据库系统。

---

> "I must not fear. Fear is the mind-killer. Fear is the little-death that brings total obliteration." - Bene Gesserit Litany Against Fear