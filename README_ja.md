# Melange DB 🪐

> sledアーキテクチャに基づく次世代高性能組み込みデータベース

[![Crates.io](https://img.shields.io/crates/v/melange_db.svg)](https://crates.io/crates/melange_db)
[![Documentation](https://docs.rs/melange_db/badge.svg)](https://docs.rs/melange_db)
[![License](https://img.shields.io/badge/license-LGPLv3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.en.html)

## 🌍 言語バージョン
- [中文版](README.md) | [English](README_en.md) | [日本語版](README_ja.md)

## プロジェクト紹介

Melange DBは、sledアーキテクチャをベースに深いパフォーマンス最適化を行った組み込みデータベースです。RocksDBのパフォーマンスを超越することを目指し、SIMD命令最適化、スマートキャッシュシステム、ブルームフィルターなどの技術により、究極の読み書きパフォーマンスを実現します。

### 🎭 創作インスピレーション

プロジェクト名と設計哲学は、フランク・ハーバートの古典SF小説「デューン」に深くインスパイアされています：

- **メランジュ（スパイス）**: デューン宇宙で最も貴重な物質、宇宙航行の鍵であり、データの価値を象徴
- **恐怖は思考殺し**: 「恐怖は心の殺し屋だ」という古典的なセリフのように、パフォーマンスへの恐怖を排除し、究極の最適化を追求
- **スパイスルート**: デューンのスパイス輸送ルートのように、Melange DBは効率的なデータフローとストレージパスを構築
- **フリーメンの精神**: 砂漠のサバイバル専門家、リソース制約環境での究極のパフォーマンス最適化を表現

このインスピレーションは、私たちの中核哲学を反映しています：**限られたリソースから無限の価値を創造する**。

## コア機能

### 🚀 究極のパフォーマンス最適化
- **SIMD最適化Key比較**: ARM64 NEON命令セットに基づく高性能比較
- **多段ブロックキャッシュシステム**: ホット/ウォーム/コールド3階層キャッシュ、LRU削除戦略
- **スマートブルームフィルター**: 1%の偽陽性率、存在しないクエリの高速フィルタリング
- **プリフェッチメカニズム**: インテリジェントなプリフェッチアルゴリズムがシーケンシャルアクセスパフォーマンスを向上

### 🔒 並行安全性
- **ロックフリーデータ構造**: concurrent-mapに基づく高並行性設計
- **スレッド安全性**: 完全なSend + Syncトレイト実装
- **原子性保証**: ACID互換トランザクションサポート

### 📦 効率的なメモリ管理
- **インクリメンタルシリアライゼーション**: I/Oオーバーヘッドを削減するシリアライゼーション戦略
- **スマートキャッシュ戦略**: 適応的キャッシュ置換アルゴリズム
- **メモリマッピング最適化**: 効率的なファイルマッピングメカニズム

## クイックスタート

### 基本的な使用方法

```rust
use melange_db::{Db, Config};

fn main() -> anyhow::Result<()> {
    // データベースを設定
    let config = Config::new()
        .path("/path/to/database")
        .cache_capacity_bytes(512 * 1024 * 1024); // 512MBキャッシュ

    // データベースを開く
    let db: Db<1024> = config.open()?;

    // データを書き込む
    let tree = db.open_tree("my_tree")?;
    tree.insert(b"key", b"value")?;

    // データを読み込む
    if let Some(value) = tree.get(b"key")? {
        println!("Found value: {:?}", value);
    }

    // 範囲クエリ
    for kv in tree.range(b"start"..b"end") {
        let (key, value) = kv?;
        println!("{}: {:?}", String::from_utf8_lossy(&key), value);
    }

    Ok(())
}
```

### 圧縮設定

Melange DBはコンパイル時機能による圧縮アルゴリズムの選択をサポートしています：

#### 無圧縮（デフォルト、最高パフォーマンス）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::None);
```

#### LZ4圧縮（パフォーマンスと圧縮率のバランス）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::Lz4);
```

#### Zstd圧縮（高圧縮率）
```rust
use melange_db::{Db, Config, CompressionAlgorithm};

let config = Config::new()
    .path("/path/to/database")
    .compression_algorithm(CompressionAlgorithm::Zstd);
```

### ビルドコマンド

```bash
# 無圧縮（デフォルト）
cargo build --release

# LZ4圧縮
cargo build --release --features compression-lz4

# Zstd圧縮
cargo build --release --features compression-zstd
```

## パフォーマンスハイライト

### Apple M1パフォーマンス
- **無圧縮**: 1.07 µs/書き込み、0.36 µs/読み取り
- **LZ4圧縮**: 0.97 µs/書き込み、0.36 µs/読み取り
- **Zstd圧縮**: 1.23 µs/書き込み、0.40 µs/読み取り

### プラットフォーム最適化
- **ARM64 NEON最適化**: Apple Silicon M1 NEON命令セットの完全活用
- **x86_64 SSE2/AVX2**: ローエンドからハイエンドまでフルカバー
- **適応的最適化**: ハードウェア特性に基づくスマート設定

## インストール

`Cargo.toml`に追加：

```toml
[dependencies]
melange_db = "0.1.5"
```

## サンプル

Melange DBをより良く使用するためのいくつかのサンプルを提供しています：

### 📊 パフォーマンステストサンプル
- **`performance_demo.rs`** - 基本的なパフォーマンスデモとスマートフラッシュ戦略のショーケース
- **`accurate_timing_demo.rs`** - P50/P95/P99統計を含む正確なタイミング分析

### 🎯 圧縮サンプル
- **`macbook_air_m1_compression_none.rs`** - 無圧縮究極パフォーマンス
- **`macbook_air_m1_compression_lz4.rs`** - NEONアクセラレーションLZ4圧縮
- **`macbook_air_m1_compression_zstd.rs`** - 高圧縮率Zstd

### 🎯 ベストプラクティスサンプル
- **`best_practices.rs`** - 完全な本番環境使用例

### サンプルの実行

```bash
# 基本的なパフォーマンスデモを実行
cargo run --example performance_demo

# 正確なタイミング分析を実行
cargo run --example accurate_timing_demo

# 圧縮アルゴリズムパフォーマンス比較を実行
cargo run --example macbook_air_m1_compression_none --features compression-none --release
cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release
cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release
```

## ライセンス

このプロジェクトはLGPL-3.0ライセンスの下で提供されています - 詳細は[LICENSE](LICENSE)ファイルを参照してください。

## コントリビューション

コントリビューションを歓迎します！お気軽にプルリクエストを提出してください。

## 謝辞

- 優れた[sled](https://github.com/spacejam/sled)データベースアーキテクチャに基づいています
- フランク・ハーバートの「デューン」宇宙にインスパイアされています
- フィードバックと提案を提供してくれたすべてのコントリビューターとユーザーに感謝します