# Melange DB 🪐

> 基于 sled 架构深度优化的下一代高性能嵌入式数据库

[![Crates.io](https://img.shields.io/crates/v/melange_db.svg)](https://crates.io/crates/melange_db)
[![Documentation](https://docs.rs/melange_db/badge.svg)](https://docs.rs/melange_db)
[![License](https://img.shields.io/badge/license-LGPLv3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.en.html)

## 🌍 语言版本
- [中文版](README.md) | [English](README_en.md) | [日本語版](README_ja.md)

## 项目简介

Melange DB 是一个基于 sled 架构进行深度性能优化的嵌入式数据库，专注于超越 RocksDB 的性能表现。通过 SIMD 指令优化、智能缓存系统和布隆过滤器等技术，实现极致的读写性能。

### 🎭 创意来源

项目名称和设计理念深受弗兰克·赫伯特的经典科幻小说《沙丘》(Dune) 启发：

- **Melange (美琅脂)**: 沙丘宇宙中最珍贵的物质，是宇宙航行的关键，象征着数据的珍贵和价值
- **恐惧是思维杀手**: 正如沙丘中的经典台词 "I must not fear. Fear is the mind-killer"，我们的设计哲学是消除对性能的恐惧，追求极致优化
- **香料之路**: 如同沙丘中的香料运输路线，Melange DB 构建了高效的数据流和存储路径
- **弗雷曼人精神**: 沙漠中的生存专家，代表着在资源受限环境下的极致性能优化

这种灵感来源反映了我们对数据库设计的核心理念：**在有限的资源中创造无限的价值**。

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
    let tree = db.open_tree("my_tree")?;
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

### 压缩配置

Melange DB 支持通过编译时特性选择压缩算法，以适应不同的性能需求：

#### 无压缩模式（默认，最佳性能）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

fn main() -> anyhow::Result<()> {
    // 无压缩配置 - 追求极致性能
    let config = Config::new()
        .path("/path/to/database")
        .compression_algorithm(CompressionAlgorithm::None)  // 无压缩
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB 缓存

    let db: Db<1024> = config.open()?;

    // 读写操作将获得最佳性能，无压缩开销
    let tree = db.open_tree("high_performance_tree")?;
    tree.insert(b"key", b"value")?;

    Ok(())
}
```

#### LZ4压缩模式（平衡性能与压缩率）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

fn main() -> anyhow::Result<()> {
    // LZ4压缩配置 - 平衡性能和存储效率
    let config = Config::new()
        .path("/path/to/database")
        .compression_algorithm(CompressionAlgorithm::Lz4)  // LZ4压缩
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB 缓存

    let db: Db<1024> = config.open()?;

    // 获得良好的压缩率，同时保持较高性能
    let tree = db.open_tree("balanced_tree")?;
    tree.insert(b"key", b"value")?;

    Ok(())
}
```

#### Zstd压缩模式（高压缩率）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

fn main() -> anyhow::Result<()> {
    // Zstd压缩配置 - 追求最大压缩率
    let config = Config::new()
        .path("/path/to/database")
        .compression_algorithm(CompressionAlgorithm::Zstd)  // Zstd压缩
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB 缓存

    let db: Db<1024> = config.open()?;

    // 获得最高的压缩率，适合存储受限场景
    let tree = db.open_tree("storage_efficient_tree")?;
    tree.insert(b"key", b"value")?;

    Ok(())
}
```

#### 特性选择和验证
```rust
use melange_db::config::CompressionAlgorithm;

fn main() -> anyhow::Result<()> {
    // 检查启用的压缩特性
    let features = CompressionAlgorithm::detect_enabled_features();
    println!("启用的压缩特性: {:?}", features);

    // 验证特性配置
    if let Some(warning) = CompressionAlgorithm::validate_feature_config() {
        println!("警告: {}", warning);
    }

    // 获取实际使用的算法
    let (algorithm, reason) = CompressionAlgorithm::get_active_algorithm_with_reason();
    println!("使用压缩算法: {:?}, 原因: {}", algorithm, reason);

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
    let users_tree = db.open_tree("users")?;
    let sessions_tree = db.open_tree("sessions")?;

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

### ⚠️ 重要说明

**示例代码优化目标**: 当前示例主要针对 Apple M1 等高端 ARM64 设备优化，配置了较大的缓存（1GB）和适用于高性能场景的参数。

**低端设备优化建议**: 如果您需要在 Intel Celeron J1800 等低端 x86 设备上运行，请参考：
- **测试文件**: `tests/low_end_x86_perf_test.rs`
- **专用配置**: 针对 2GB 内存和 SSE2 指令集优化
- **性能目标**: 写入 9-15 µs/条，读取 2-5 µs/条

### 运行示例

```bash
# 运行基本性能演示
cargo run --example performance_demo

# 运行精确计时分析
cargo run --example accurate_timing_demo

# 运行最佳实践示例
cargo run --example best_practices

# 运行压缩算法性能对比（需要指定压缩特性）
cargo run --example macbook_air_m1_compression_none --features compression-none --release
cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release
cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release
```

### 低端设备配置参考

```rust
// 针对 Intel Celeron J1800 + 2GB 内存的优化配置
let mut config = Config::new()
    .path("low_end_db")
    .flush_every_ms(None)  // 禁用传统自动flush，使用智能flush
    .cache_capacity_bytes(32 * 1024 * 1024);  // 32MB缓存，适应2GB内存

// 优化智能flush配置
config.smart_flush_config = crate::smart_flush::SmartFlushConfig {
    enabled: true,
    base_interval_ms: 100,     // 100ms基础间隔
    min_interval_ms: 30,        // 30ms最小间隔
    max_interval_ms: 1500,     // 1.5s最大间隔
    write_rate_threshold: 4000, // 4K ops/sec阈值
    accumulated_bytes_threshold: 2 * 1024 * 1024, // 2MB累积阈值
};
```

### 示例亮点

✅ **配置优化**: 展示如何根据应用场景调整缓存和 flush 参数
✅ **数据建模**: 演示结构化数据的存储和查询模式
✅ **批量操作**: 介绍高效的数据插入和处理技巧
✅ **查询优化**: 展示范围查询和前缀查询的最佳实践
✅ **事务处理**: 演示如何保证数据一致性
✅ **性能监控**: 提供性能统计和监控建议

## 性能表现

### 性能测试数据

详细的性能测试数据和硬件环境信息请查看：[docs/test_data/index.md](docs/test_data/index.md)

### 性能亮点

- **高端设备表现**: 在Apple M1上达到写入1.23 µs/条，读取0.42 µs/条的优异性能
- **低端设备优化**: 在Intel Celeron J1800上通过智能flush优化实现写入9.13 µs/条，读取2.56 µs/条
- **低功耗设备适配**: 在树莓派3B+上实现写入39.04 µs/条，读取9.06 µs/条，成功适配1GB内存+SD卡存储环境
- **对比优势**: 相比RocksDB最高提升4倍写入性能
- **跨平台支持**: 优化了ARM64和x86_64架构的性能表现

### 测试覆盖

- **硬件多样性**: 从高端Apple M1、低端Intel Celeron到树莓派3B+的完整测试覆盖
- **系统兼容**: macOS和Linux多平台验证
- **优化验证**: SIMD指令集、智能flush、缓存策略等多维度优化效果验证
- **持续测试**: 定期性能回归测试确保性能稳定性

## 优化技术详解

### 1. SIMD 优化
- **多平台支持**: 同时支持 ARM64 NEON 和 x86_64 SSE2/AVX2 指令集
- **ARM64 NEON**: 支持 Apple M1 和树莓派 3b+ 等ARM64设备
- **x86_64 SSE2**: 支持Intel Celeron等不支持AVX2的低端x86设备
- **x86_64 AVX2**: 支持现代Intel/AMD处理器，32字节向量处理
- **自适应检测**: 运行时自动检测CPU支持的指令集并选择最优实现
- **小key优化**: 针对≤16字节的key使用快速64位整数比较
- **批量处理**: 支持批量key比较操作，提升缓存命中率
- **降级策略**: 不支持SIMD时使用优化的标量比较算法

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
   - 小型应用/低端设备: 32MB - 256MB
   - 中型应用: 512MB - 1GB
   - 大型应用/高性能设备: 2GB - 4GB+

2. **智能 Flush 策略**
   - 启用智能 flush 以平衡性能和数据安全
   - 根据写入负载调整 flush 间隔
   - 设置合理的累积字节阈值
   - **低端设备优化**: 减少基础间隔，提高频率，降低累积阈值
   - **树莓派3B+特殊优化**: 增加flush间隔到200ms，降低写入阈值到2000 ops/sec，适应SD卡存储特性

3. **树的设计**
   - 使用有意义的树名
   - 合理设计键的前缀结构
   - 避免单个树过大

4. **硬件适配建议**
   - **高端设备 (Apple M1等)**: 使用示例中的1GB缓存配置
   - **低端设备 (Intel Celeron等)**: 参考 `tests/low_end_x86_perf_test.rs` 配置
   - **树莓派3B+等低功耗ARM设备**: 参考 `tests/raspberry_pi_perf_test.rs` 配置
   - **内存受限环境**: 减少缓存大小，优化flush策略，考虑使用增量序列化

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
- [x] 智能预取算法（基于访问模式）
- [x] 完整的多平台SIMD支持（ARM64 NEON + x86_64 SSE2/AVX2）
- [x] LZ4压缩算法支持（特性可选，提供更快的压缩速度）
- [x] 多压缩算法特性选择（无压缩/LZ4/Zstd，编译时特性控制）
- [x] 压缩算法性能优化和测试数据收集
- [x] Apple Silicon M1专项优化和性能验证

### 🔄 进行中
- [ ] 自适应压缩策略（运行时根据数据特征选择压缩算法）

### 📋 下一阶段开发计划（v0.2.0）

#### 🎯 性能优化
- [x] **自适应压缩策略** - 运行时根据数据特征自动选择最优压缩算法
- [ ] **热点数据识别** - 智能识别高频访问数据，优化缓存策略
- [ ] **内存压缩** - 对冷数据进行内存压缩，提高内存利用率
- [ ] **IO优化** - 异步IO支持，减少阻塞等待时间
- [ ] **批量操作优化** - 进一步优化批量插入和查询性能

#### 🛠️ 功能增强
- [ ] **事务支持** - 完整的ACID事务支持
- [ ] **范围查询优化** - 更高效的范围查询和迭代器
- [ ] **二级索引** - 支持创建和管理二级索引
- [ ] **数据TTL** - 自动过期数据清理
- [ ] **备份恢复** - 数据备份和恢复机制

#### 🔧 架构改进
- [ ] **插件系统** - 可扩展的插件架构
- [ ] **配置管理** - 更灵活的配置系统
- [ ] **监控指标** - 详细的性能监控和指标收集
- [ ] **日志系统** - 结构化日志和审计功能

#### 🌐 平台支持
- [ ] **Windows优化** - Windows平台专项优化
- [ ] **Android支持** - Android平台适配
- [ ] **iOS支持** - iOS平台适配
- [ ] **WebAssembly** - WASM支持，浏览器环境使用

#### 📊 工具生态
- [ ] **CLI工具** - 命令行管理工具
- [ ] **Web界面** - 基于Web的管理界面
- [ ] **性能分析** - 性能分析和调优工具
- [ ] **数据迁移** - 数据导入导出工具

### 🚀 长期规划（v1.0+）

#### 分布式版本
- [ ] **集群支持** - 多节点集群部署
- [ ] **数据分片** - 自动数据分片和负载均衡
- [ ] **故障转移** - 自动故障检测和转移
- [ ] **一致性协议** - 分布式一致性保证

#### 高级查询优化
- [ ] **查询计划** - 智能查询计划优化
- [ ] **索引优化** - 自适应索引策略
- [ ] **缓存策略** - 多级缓存优化
- [ ] **预计算** - 查询结果预计算

#### 机器学习集成
- [ ] **访问模式预测** - 基于ML的访问模式预测
- [ ] **自动调优** - 基于ML的参数自动调优
- [ ] **异常检测** - 数据访问异常检测
- [ ] **容量规划** - 智能容量规划建议

## 技术栈

### 开发环境
- **核心语言**: Rust 1.70+
- **基础架构**: 基于 sled 代码库
- **并发控制**: concurrent-map, parking_lot
- **SIMD 优化**:
  - std::arch::aarch64 (ARM64 NEON指令集)
  - std::arch::x86_64 (x86_64 SSE2/AVX2指令集)
- **压缩**: zstd, lz4_flex（特性可选）
- **测试**: criterion, tokio-test

### 压缩算法选择（编译时特性）

Melange DB 支持通过编译时特性选择压缩算法，以适应不同的性能需求：

#### 无压缩（默认）
```bash
cargo build --release
```
- **适用场景**: 低端设备（如树莓派）、追求极致性能
- **特点**: 零CPU开销，最快读写速度

#### LZ4压缩
```bash
cargo build --release --features compression-lz4
```
- **适用场景**: 中等性能设备，需要较好的压缩速度和压缩率平衡
- **特点**: 极快的压缩/解压缩速度，适度压缩率

#### Zstd压缩
```bash
cargo build --release --features compression-zstd
```
- **适用场景**: 存储空间受限，需要高压缩率
- **特点**: 高压缩率，但CPU开销相对较大

#### 特性优先级
如果同时启用多个压缩特性，将按以下优先级选择：**none > lz4 > zstd**

#### 配置验证
```rust
use melange_db::config::CompressionAlgorithm;

// 检查启用的压缩特性
let features = CompressionAlgorithm::detect_enabled_features();
println!("启用的压缩特性: {:?}", features);

// 验证特性配置
if let Some(warning) = CompressionAlgorithm::validate_feature_config() {
    println!("警告: {}", warning);
}

// 获取实际使用的算法
let (algorithm, reason) = CompressionAlgorithm::get_active_algorithm_with_reason();
println!("使用压缩算法: {:?}, 原因: {}", algorithm, reason);
```

### 目标平台
- **ARM64平台**: Apple M1, Raspberry Pi 3b+ 等ARM64设备
- **x86_64平台**: Intel/AMD处理器，从低端到高端全覆盖
- **编译目标**:
  - aarch64-apple-darwin, aarch64-unknown-linux-gnu
  - x86_64-apple-darwin, x86_64-unknown-linux-gnu
- **指令集支持**:
  - ARM64: ARMv8.4-A with NEON SIMD
  - x86_64: SSE2 (基础), AVX2 (现代处理器)
- **运行时检测**: 自动检测CPU特性并选择最优SIMD实现

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目采用 GNU Lesser General Public License v3.0 (LGPLv3) 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

### 许可证说明

选择 LGPLv3 的原因：

- **商业友好**: 允许将 Melange DB 用于商业软件和闭源项目
- **动态链接**: 通过动态链接方式使用时，主程序可以保持闭源
- **开源义务**: 仅对库本身的修改需要开源，不影响使用方代码
- **社区贡献**: 确保改进和优化能够回馈给开源社区

### 使用场景

- ✅ **商业软件**: 可以在闭源商业软件中使用
- ✅ **开源项目**: 完全兼容其他开源许可证
- ✅ **动态链接**: 推荐的使用方式，主程序保持闭源
- ✅ **修改贡献**: 对 Melange DB 的改进需要开源

## 致谢

- 基于优秀的 [sled](https://github.com/spacejam/sled) 数据库架构
- 灵感来自弗兰克·赫伯特的《沙丘》宇宙
- 感谢所有提供反馈和建议的贡献者和用户

> "I must not fear. Fear is the mind-killer." - Frank Herbert, Dune