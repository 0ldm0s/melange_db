# Melange DB 🪐

> Next-generation high-performance embedded database with deep optimizations based on sled architecture

[![Crates.io](https://img.shields.io/crates/v/melange_db.svg)](https://crates.io/crates/melange_db)
[![Documentation](https://docs.rs/melange_db/badge.svg)](https://docs.rs/melange_db)
[![License](https://img.shields.io/badge/license-LGPLv3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.en.html)

## 🌍 Language Versions
- [中文版](README.md) | [English](README_en.md) | [日本語版](README_ja.md)

## Project Introduction

Melange DB is an embedded database built on the sled architecture with deep performance optimizations, focused on exceeding RocksDB's performance. Through technologies like SIMD instruction optimization, intelligent caching systems, and Bloom filters, it achieves extreme read/write performance.

### 🎭 Creative Inspiration

The project name and design philosophy are deeply inspired by Frank Herbert's classic science fiction novel "Dune":

- **Melange**: The most precious substance in the Dune universe, essential for space travel, symbolizing the value of data
- **Fear is the mind-killer**: Like the classic line "I must not fear. Fear is the mind-killer", our design philosophy eliminates fear of performance, pursuing ultimate optimization
- **Spice route**: Like the spice transportation routes in Dune, Melange DB builds efficient data flow and storage paths
- **Fremen spirit**: Survival experts in the desert, representing extreme performance optimization in resource-constrained environments

This inspiration reflects our core philosophy: **Creating infinite value from limited resources**.

## Core Features

### 🚀 Extreme Performance Optimization
- **SIMD-optimized Key comparison**: High-performance comparison based on ARM64 NEON instruction set
- **Multi-level block cache system**: Hot/warm/cold three-level cache with LRU eviction strategy
- **Smart Bloom filters**: 1% false positive rate, rapid filtering of non-existent queries
- **Prefetch mechanism**: Intelligent prefetch algorithms improve sequential access performance

### 🔒 Concurrency Safety
- **Lock-free data structures**: High-concurrency design based on concurrent-map
- **Thread safety**: Complete Send + Sync trait implementation
- **Atomic guarantees**: ACID-compatible transaction support

### 🔥 Atomic Operations Unified Architecture (Major Performance Upgrade)

> **Version v0.2.0**: Introduces a brand-new atomic operations unified architecture, completely resolving EBR conflicts in high-concurrency scenarios.

#### 🚀 Breaking Upgrade Notice

**This is a breaking performance upgrade** with the following major improvements:

✅ **Resolved Issues**:
- **EBR RefCell Conflicts**: Completely eliminated `RefCell already borrowed` panics during multi-threaded high-concurrency operations
- **Data Races**: Eliminated race conditions between atomic operations and database operations
- **Performance Bottlenecks**: Significantly improved concurrent performance through inter-worker communication

⚠️ **API Changes**:
- `atomic_operations_manager::AtomicOperationsManager` - Brand new unified router design
- `atomic_worker::AtomicWorker` - Refactored to completely independent atomic operations component
- `database_worker::DatabaseWorker` - New dedicated database operations Worker

#### 🏗️ New Architecture Design

**SegQueue Unified Architecture**:
```
AtomicOperationsManager (Pure Router)
    ├── SegQueue A ↔ AtomicWorker (DashMap + AtomicU64)
    │   └── Auto-sends persistence commands → DatabaseWorker queue
    └── SegQueue B ↔ DatabaseWorker (All database operations)
```

#### ✅ Core Advantages

1. **Complete Decoupling**:
   - AtomicOperationsManager only handles routing, doesn't operate any data structures
   - AtomicWorker specializes in atomic operations, doesn't directly access database
   - DatabaseWorker specializes in all database operations

2. **Inter-Worker Communication**:
   - AtomicWorker automatically sends persistence commands to DatabaseWorker after operations
   - Completely avoids EBR conflicts in the same thread

3. **Unified SegQueue Usage**:
   - All Workers use the same concurrent queue mechanism
   - Maintains consistency with existing architecture

#### 📊 Performance Validation

**12-thread high-pressure test results**:
- ✅ **285 atomic operations**: 160 + 50 + 40 + 35 page accesses
- ✅ **570 database records**: 300 + 150 + 120 records
- ✅ **Zero EBR conflicts**: 12 threads running simultaneously completely safe
- ✅ **100% data consistency**: All counters and record data completely accurate

#### 🚀 Quick Start

Want to get started with Melange DB quickly? Check out the following latest example files:

**Hybrid Manager Architecture (Recommended)**:
- `cargo run --example hybrid_manager_guide` - Complete usage tutorial
- `cargo run --example hybrid_best_practices` - Production environment best practices

**Performance Testing**:
- `cargo run --example high_pressure_segqueue_test` - High concurrency stress test
- `cargo run --example performance_demo` - Basic performance demonstration

All example files contain detailed code comments and usage instructions to ensure you can quickly understand and use Melange DB.

#### 🧪 Test Cases

```bash
# High-pressure concurrent test (12 threads)
cargo run --example high_pressure_segqueue_test

# Hybrid Manager Best Practices
cargo run --example hybrid_best_practices

# Hybrid Manager Usage Guide
cargo run --example hybrid_manager_guide
```

#### 🔄 Migration Guide

**Old Version (v0.1.4 and below)**:
```rust
// ❌ Deprecated - causes EBR conflicts
let db = Arc::new(config.open()?);
// Direct multi-threaded db operations cause RefCell conflicts
```

**New Version (v0.2.0+)**:
```rust
// ✅ Recommended - No EBR conflicts
let manager = Arc::new(AtomicOperationsManager::new(Arc::new(config.open()?)));
// Operations through unified router, completely thread-safe
```

📖 **Detailed Migration Guide**: See [docs/migration_guide_v0.2.0_en.md](docs/migration_guide_v0.2.0_en.md) for complete upgrade steps and troubleshooting guide.

#### ⚡ Performance Improvements

- **Concurrent Safety**: Supports unlimited concurrent threads
- **Zero Conflicts**: Completely eliminates EBR RefCell borrowing issues
- **Auto Persistence**: Atomic operations automatically persist after completion
- **Data Consistency**: Ensures data integrity under high concurrency

### 📦 Efficient Memory Management
- **Incremental serialization**: Serialization strategy reducing IO overhead
- **Smart caching strategy**: Adaptive cache replacement algorithms
- **Memory mapping optimization**: Efficient file mapping mechanism

## Quick Start

### 📚 Learning Path

**Recommended learning order for new users**:

1. **Getting Started Tutorial**: `cargo run --example hybrid_manager_guide`
   - Learn basic usage of the hybrid manager
   - Understand the unified interface for atomic operations and database operations
   - Master data persistence and counter usage

2. **Best Practices**: `cargo run --example hybrid_best_practices`
   - Learn production environment best practices
   - Master practical scenarios like user management and session handling
   - Understand performance optimization and error handling

3. **Performance Testing**: `cargo run --example performance_demo`
   - Learn about Melange DB's performance characteristics
   - Understand cache configuration and flush strategies
   - Master performance monitoring methods

4. **Advanced Features**: `cargo run --example rat_logger_demo`
   - Learn logging system integration
   - Understand debugging and monitoring methods

All example files are complete, runnable programs with detailed English comments to help you quickly master Melange DB's various features.

### Compression Configuration

Melange DB supports compression algorithm selection through compile-time features:

#### No Compression (Default, Best Performance)
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::None);
```

#### LZ4 Compression (Balanced Performance and Compression)
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::Lz4);
```

#### Zstd Compression (High Compression Ratio)
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::Zstd);
```

### Build Commands

```bash
# No compression (default)
cargo build --release

# LZ4 compression
cargo build --release --features compression-lz4

# Zstd compression
cargo build --release --features compression-zstd
```

## Performance Highlights

### Apple M1 Performance
- **No compression**: 1.07 µs/op write, 0.36 µs/op read
- **LZ4 compression**: 0.97 µs/op write, 0.36 µs/op read
- **Zstd compression**: 1.23 µs/op write, 0.40 µs/op read

### Platform Optimization
- **ARM64 NEON optimization**: Full utilization of Apple Silicon M1 NEON instruction set
- **x86_64 SSE2/AVX2**: From low-end to high-end full coverage
- **Adaptive optimization**: Smart configuration based on hardware characteristics

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
melange_db = "0.2.0"
```

## Examples

### 🔥 Available Example Overview

**Hybrid Manager Architecture (Recommended)**:
- `cargo run --example hybrid_manager_guide` - Complete usage tutorial and API introduction
- `cargo run --example hybrid_best_practices` - Production environment best practices
- `cargo run --example high_pressure_segqueue_test` - 12-thread high concurrency stress test

**Performance Testing and Analysis**:
- `cargo run --example performance_demo` - Basic performance demonstration
- `cargo run --example accurate_timing_demo` - Precise timing analysis (P50/P95/P99)
- `cargo run --example best_practices` - Traditional API best practices

**System Integration**:
- `cargo run --example rat_logger_demo` - Logging system integration
- `cargo run --example no_logger_test` - No logger environment test

**Platform Performance Testing**:
- `cargo run --example macbook_air_m1_compression_none --features compression-none --release`
- `cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release`
- `cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release`

### 📊 Performance and Functionality Testing
- **Performance Benchmark Test**: `cargo run --example performance_demo`
  - Basic performance demonstration and smart flush strategy showcase
  - Includes read/write performance statistics and cache hit rate analysis

- **Precise Timing Analysis**: `cargo run --example accurate_timing_demo`
  - Detailed performance analysis with P50/P95/P99 statistics
  - Shows latency distribution for different operation types

- **Best Practices Demonstration**: `cargo run --example best_practices`
  - Complete production environment usage example
  - Includes user data management, session handling, transaction operations, etc.

- **Logging System Integration**: `cargo run --example rat_logger_demo`
  - Shows how to integrate rat_logger high-performance logging system
  - Demonstrates log configuration and performance debug output

- **No Logger Test**: `cargo run --example no_logger_test`
  - Verifies safe behavior when logger is not initialized
  - Shows library backward compatibility

### 🖥️ Platform Performance Testing
- **M1 MacBook Air Performance Test**:
  ```bash
  # No compression version (best performance)
  cargo run --example macbook_air_m1_compression_none --features compression-none --release

  # LZ4 compression version (balanced performance)
  cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release

  # Zstd compression version (high compression ratio)
  cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release
  ```

### ⚠️ Deprecated Examples (v0.1.4 and below)
- `simple_atomic_sequence` - Migrated to new unified architecture
- `atomic_operations_test` - Has EBR conflict issues, deprecated
- `atomic_mixed_operations` - Has concurrency limitations, deprecated

### 🔄 Migration Suggestions

**It is recommended to use the latest hybrid manager architecture**:
```bash
# Learn basic usage
cargo run --example hybrid_manager_guide

# Production environment reference
cargo run --example hybrid_best_practices

# Performance stress testing
cargo run --example high_pressure_segqueue_test
```

### 📊 Performance Testing Examples
- **`performance_demo.rs`** - Basic performance demonstration and smart flush strategy showcase
- **`accurate_timing_demo.rs`** - Precise timing analysis with P50/P95/P99 statistics

### 🎯 Compression Examples
- **`macbook_air_m1_compression_none.rs`** - No compression extreme performance
- **`macbook_air_m1_compression_lz4.rs`** - NEON-accelerated LZ4 compression
- **`macbook_air_m1_compression_zstd.rs`** - High compression ratio Zstd

### 🎯 Best Practice Examples
- **`best_practices.rs`** - Complete production environment usage example

### 📝 Logging System Integration Example
- **`rat_logger_demo.rs`** - Shows how to integrate rat_logger logging system

### ⚠️ Safety Guarantee

**Behavior when logging is not initialized**:
- ✅ **Completely Safe**: All logging calls are silently ignored if rat_logger is not initialized
- ✅ **Zero Exceptions**: No panics, errors, or runtime exceptions will occur
- ✅ **Normal Execution**: Programs run completely normally, just without log output
- ✅ **Zero Overhead**: Debug level logs are zero-cost in release mode anyway
- ✅ **Backward Compatible**: Old code works fine without logger initialization

This design ensures:
- **Gradual Adoption**: Selectively enable logging for specific modules
- **Production Friendly**: Completely zero overhead when logging is not needed
- **Caller Full Control**: Callers decide whether logging functionality is needed

```rust
// Code works fine even without initializing logging
use melange_db::{Db, Config};

fn main() -> anyhow::Result<()> {
    // Note: rat_logger is not initialized here!

    let config = Config::new()
        .path("example_db")
        .cache_capacity_bytes(1024 * 1024);

    let db: Db<1024> = config.open()?;
    let tree = db.open_tree("my_tree")?;
    tree.insert(b"key", b"value")?;  // Log calls are silently ignored
    Ok(())
}
```

### Running Examples

```bash
# Hybrid Manager Architecture (Recommended)
cargo run --example hybrid_manager_guide
cargo run --example hybrid_best_practices

# Performance Testing
cargo run --example performance_demo
cargo run --example accurate_timing_demo
cargo run --example high_pressure_segqueue_test

# System Integration
cargo run --example rat_logger_demo
cargo run --example no_logger_test

# Platform Performance Testing
cargo run --example macbook_air_m1_compression_none --features compression-none --release
cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release
cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release
```

## License

This project is licensed under the LGPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Based on the excellent [sled](https://github.com/spacejam/sled) database architecture
- Inspired by Frank Herbert's "Dune" universe
- Thanks to all contributors and users who provide feedback and suggestions