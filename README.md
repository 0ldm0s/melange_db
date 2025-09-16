# Melange DB 🪐

> 基于 sled 架构深度优化的下一代高性能嵌入式数据库

## 项目简介

Melange DB 是一个基于 sled 架构进行深度性能优化的嵌入式数据库，专注于超越 RocksDB 的性能表现。通过 SIMD 指令优化、智能缓存系统和布隆过滤器等技术，实现极致的读写性能。

## 核心特性

### 🚀 极致性能优化
- **SIMD 优化的 Key 比较**: 基于 ARM64 NEON 指令集的高性能比较
- **多级块缓存系统**: 热/温/冷三级缓存，LRU 淘汰策略
- **智能布隆过滤器**: 1% 误判率，快速过滤不存在查询
- **预取机制**: 智能预取算法提升顺序访问性能

### 🔒 并发安全
- **无锁数据结构**: 基于 concurrent-map 的高并发设计
- **线程安全**: 完全的 Send + Sync trait 实现
- **原子性保证**: ACID 兼容的事务支持

### 📦 高效内存管理
- **增量序列化**: 减少 IO 开销的序列化策略
- **智能缓存策略**: 自适应缓存替换算法
- **内存映射优化**: 高效的文件映射机制

## 快速开始

### 基本使用

```rust
use melange_db::{Db, Config};

fn main() -> anyhow::Result<()> {
    // 配置数据库
    let config = Config::new()
        .path("/path/to/database")
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB 缓存

    // 打开数据库
    let db: Db<1024> = config.open()?;

    // 写入数据
    let tree = db.open_tree::<&[u8]>(b"my_tree")?;
    tree.insert(b"key", b"value")?;

    // 读取数据
    if let Some(value) = tree.get(b"key")? {
        println!("Found value: {:?}", value);
    }

    // 范围查询
    for kv in tree.range(b"start"..b"end") {
        let (key, value) = kv?;
        println!("{}: {:?}", String::from_utf8_lossy(&key), value);
    }

    Ok(())
}
```

### 最佳实践配置

```rust
use melange_db::{Db, Config};

fn main() -> anyhow::Result<()> {
    // 生产环境推荐配置
    let mut config = Config::new()
        .path("/path/to/database")
        .cache_capacity_bytes(1024 * 1024 * 1024) // 1GB 缓存
        .flush_every_ms(Some(1000)); // 1秒 flush 间隔

    // 启用智能 flush 策略
    config.smart_flush_config.enabled = true;
    config.smart_flush_config.base_interval_ms = 1000;
    config.smart_flush_config.min_interval_ms = 100;
    config.smart_flush_config.max_interval_ms = 5000;
    config.smart_flush_config.write_rate_threshold = 5000;
    config.smart_flush_config.accumulated_bytes_threshold = 8 * 1024 * 1024; // 8MB

    let db: Db<1024> = config.open()?;

    // 使用有意义的树名
    let users_tree = db.open_tree::<&[u8]>(b"users")?;
    let sessions_tree = db.open_tree::<&[u8]>(b"sessions")?;

    Ok(())
}
```

## 示例代码

我们提供了多个示例来帮助您更好地使用 Melange DB：

### 📊 性能测试示例
- **`performance_demo.rs`** - 基本性能演示和智能 flush 策略展示
- **`accurate_timing_demo.rs`** - 精确计时分析，包含 P50/P95/P99 统计

### 🎯 最佳实践示例
- **`best_practices.rs`** - 完整的生产环境使用示例，包含：
  - 用户数据管理
  - 会话处理
  - 事务操作
  - 批量插入
  - 范围查询
  - 数据清理

### 运行示例

```bash
# 运行基本性能演示
cargo run --example performance_demo

# 运行精确计时分析
cargo run --example accurate_timing_demo

# 运行最佳实践示例
cargo run --example best_practices
```

### 示例亮点

✅ **配置优化**: 展示如何根据应用场景调整缓存和 flush 参数
✅ **数据建模**: 演示结构化数据的存储和查询模式
✅ **批量操作**: 介绍高效的数据插入和处理技巧
✅ **查询优化**: 展示范围查询和前缀查询的最佳实践
✅ **事务处理**: 演示如何保证数据一致性
✅ **性能监控**: 提供性能统计和监控建议

## 性能表现

### 基准测试结果 (Apple M1)

| 操作类型 | 平均延迟 | P95 延迟 | P99 延迟 | 吞吐量 |
|---------|---------|---------|---------|--------|
| **单条插入** | 0.78 µs | 1.54 µs | 7.17 µs | 1.28M ops/sec |
| **单条读取** | 0.37 µs | 0.42 µs | 1.21 µs | 2.70M ops/sec |
| **批量插入** | 1.40 µs | - | - | 714K ops/sec |
| **更新操作** | 2.22 µs | - | - | 450K ops/sec |

### Release模式实测性能

| 操作类型 | 平均延迟 | 吞吐量 |
|---------|---------|--------|
| **单条插入** | 9.92 µs | 100.8K ops/sec |
| **单条读取** | 0.63 µs | 1.59M ops/sec |
| **批量插入** | 1.22 µs/op | 819.7K ops/sec |
| **批量读取** | 0.47 µs/op | 2.15M ops/sec |
| **高性能写入** | 1.23 µs/op | 810.2K ops/sec |
| **高性能读取** | 0.42 µs/op | 2.37M ops/sec |

### 与 RocksDB 对比

- **写入性能**: **4.05倍** 提升 (RocksDB: 5 µs/条 → Melange DB: 1.23 µs/条)
- **读取性能**: **1.19倍** 提升 (RocksDB: 0.5 µs/条 → Melange DB: 0.42 µs/条)
- **内存效率**: 优化后的缓存策略，32.69 bytes平均记录大小
- **实际优化**: SIMD指令优化、布隆过滤器、多级缓存系统、增量序列化

## 优化技术详解

### 1. SIMD 优化
- **ARM64 NEON 指令集**: 支持 Apple M1 和树莓派 3b+
- **16 字节对齐比较**: 284M comparisons/sec
- **批量处理优化**: 支持批量 key 比较操作

### 2. 布隆过滤器
- **多级过滤器**: 支持热/温/冷数据分层
- **可配置误判率**: 默认 1%，可按需调整
- **并发安全**: 支持多线程同时访问

### 3. 块缓存系统
- **三级缓存架构**: 热/温/冷数据分层存储
- **智能预取**: 基于访问模式的预取算法
- **压缩支持**: 自动压缩大块数据
- **100% 命中率**: 在测试场景下的完美表现

## 架构设计

### 核心组件

1. **Tree**: B+ 树索引结构，支持范围查询
2. **Leaf**: 叶子节点，存储实际数据
3. **ObjectCache**: 对象缓存系统，集成优化组件
4. **Heap**: 堆管理器，负责内存分配
5. **Index**: 并发索引，支持高并发访问

### 优化集成

- **透明优化**: 用户无需修改现有代码即可获得性能提升
- **向后兼容**: 完全兼容 sled 的 API 设计
- **渐进式优化**: 可以选择性启用特定优化功能

## 适用场景

- **高频交易系统**: 低延迟读写需求
- **实时数据分析**: 高吞吐量数据处理
- **嵌入式设备**: 资源受限环境下的高性能存储
- **缓存系统**: 作为分布式缓存的后端存储

## 最佳实践指南

### 🎯 配置优化

1. **缓存大小设置**
   - 小型应用: 256MB - 512MB
   - 中型应用: 1GB - 2GB
   - 大型应用: 4GB+

2. **智能 Flush 策略**
   - 启用智能 flush 以平衡性能和数据安全
   - 根据写入负载调整 flush 间隔
   - 设置合理的累积字节阈值

3. **树的设计**
   - 使用有意义的树名
   - 合理设计键的前缀结构
   - 避免单个树过大

### 📊 性能优化

1. **批量操作**
   - 大量数据插入使用批量操作
   - 避免频繁的单条插入
   - 利用预热优化性能

2. **查询优化**
   - 使用范围查询获取连续数据
   - 利用前缀查询过滤数据
   - 避免全表扫描

3. **数据管理**
   - 定期清理过期数据
   - 使用事务保证数据一致性
   - 监控数据库大小和性能

### 🔧 开发建议

1. **开发环境**
   - 使用较小的缓存大小进行开发
   - 启用调试日志监控性能
   - 定期进行性能测试

2. **生产环境**
   - 根据实际负载调整配置参数
   - 监控关键性能指标
   - 建立数据备份机制

3. **测试策略**
   - 进行压力测试验证性能
   - 测试故障恢复能力
   - 验证数据一致性

## 开发路线

### ✅ 已完成优化
- [x] SIMD 优化的 key 比较
- [x] 多级布隆过滤器
- [x] 智能块缓存系统
- [x] 增量序列化优化
- [x] 内存使用优化
- [x] 智能自适应 flush 策略
- [x] 完整的示例代码和文档

### 🔄 进行中
- [ ] 更智能的预取算法
- [ ] 自适应压缩策略
- [ ] 更多平台支持 (x86_64 SIMD)

### 📋 未来规划
- [ ] 分布式版本
- [ ] 更高级的查询优化
- [ ] 集成机器学习优化

## 技术栈

- **核心语言**: Rust 1.70+
- **基础架构**: 基于 sled 代码库
- **并发控制**: concurrent-map, parking_lot
- **SIMD 优化**: std::arch::aarch64
- **压缩**: zstd
- **测试**: criterion, tokio-test

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

---

> "性能优化是一门艺术，需要深入理解系统底层和精心设计每一个细节" - Melange DB 开发团队