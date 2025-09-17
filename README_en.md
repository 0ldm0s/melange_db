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

### 📦 Efficient Memory Management
- **Incremental serialization**: Serialization strategy reducing IO overhead
- **Smart caching strategy**: Adaptive cache replacement algorithms
- **Memory mapping optimization**: Efficient file mapping mechanism

## Quick Start

### Basic Usage

```rust
use melange_db::{Db, Config};

fn main() -> anyhow::Result<()> {
    // Configure database
    let config = Config::new()
        .path("/path/to/database")
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MB cache

    // Open database
    let db: Db<1024> = config.open()?;

    // Write data
    let tree = db.open_tree("my_tree")?;
    tree.insert(b"key", b"value")?;

    // Read data
    if let Some(value) = tree.get(b"key")? {
        println!("Found value: {:?}", value);
    }

    // Range queries
    for kv in tree.range(b"start"..b"end") {
        let (key, value) = kv?;
        println!("{}: {:?}", String::from_utf8_lossy(&key), value);
    }

    Ok(())
}
```

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
melange_db = "0.1.3"
```

## Examples

We provide several examples to help you better use Melange DB:

### 📊 Performance Testing Examples
- **`performance_demo.rs`** - Basic performance demonstration and smart flush strategy showcase
- **`accurate_timing_demo.rs`** - Precise timing analysis with P50/P95/P99 statistics

### 🎯 Compression Examples
- **`macbook_air_m1_compression_none.rs`** - No compression extreme performance
- **`macbook_air_m1_compression_lz4.rs`** - NEON-accelerated LZ4 compression
- **`macbook_air_m1_compression_zstd.rs`** - High compression ratio Zstd

### 🎯 Best Practice Examples
- **`best_practices.rs`** - Complete production environment usage example

### Running Examples

```bash
# Run basic performance demo
cargo run --example performance_demo

# Run precise timing analysis
cargo run --example accurate_timing_demo

# Run compression algorithm performance comparison
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