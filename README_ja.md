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

### 🔥 原子操作統一アーキテクチャ（重大なパフォーマンスアップグレード）

> **バージョン v0.2.0**: 全新しい原子操作統一アーキテクチャを導入し、高並行シナリオにおけるEBR競合を完全に解決しました。

#### 🚀 破壊的アップグレード通知

**これは破壊的パフォーマンスアップグレード**で、以下の重大な改良を含みます：

✅ **解決された問題**:
- **EBR RefCell競合**: マルチスレッド高並行操作時の`RefCell already borrowed`パニックを完全に排除
- **データ競合**: 原子操作とデータベース操作間の競合状態を排除
- **パフォーマンスボトルネック**: ワーカー間通信により並行パフォーマンスを大幅向上

⚠️ **API変更**:
- `atomic_operations_manager::AtomicOperationsManager` - 全新しい統一ルーター設計
- `atomic_worker::AtomicWorker` - 完全に独立した原子操作コンポーネントに再構築
- `database_worker::DatabaseWorker` - 新しい専用データベース操作ワーカー

#### 🏗️ 新アーキテクチャ設計

**SegQueue統一アーキテクチャ**:
```
AtomicOperationsManager (純粋ルーター)
    ├── SegQueue A ↔ AtomicWorker (DashMap + AtomicU64)
    │   └── 自動永続化命令送信 → DatabaseWorkerキュー
    └── SegQueue B ↔ DatabaseWorker (すべてのデータベース操作)
```

#### ✅ コアアドバンテージ

1. **完全分離**:
   - AtomicOperationsManagerはルーティングのみを担当し、データ構造を操作しない
   - AtomicWorkerは原子操作を専門に処理し、データベースに直接アクセスしない
   - DatabaseWorkerはすべてのデータベース操作を専門に処理

2. **ワーカー間通信**:
   - AtomicWorkerは操作完了後に自動的にDatabaseWorkerに永続化命令を送信
   - 同一スレッドでのEBR競合を完全に回避

3. **統一SegQueue使用**:
   - すべてのワーカーが同じ並行キュー機構を使用
   - 既存アーキテクチャとの一貫性を維持

#### 📊 パフォーマンス検証

**12スレッド高圧力テスト結果**:
- ✅ **285回の原子操作**: 160 + 50 + 40 + 35回のページアクセス
- ✅ **570件のデータベースレコード**: 300 + 150 + 120件
- ✅ **ゼロEBR競合**: 12スレッド同時実行完全に安全
- ✅ **100%データ一貫性**: すべてのカウンターとレコードデータ完全に正確

#### 🚀 クイックスタート

Melange DBをすぐに始めたいですか？以下の最新のサンプルファイルを確認してください：

**ハイブリッドマネージャーアーキテクチャ（推奨）**：
- `cargo run --example hybrid_manager_guide` - 完全な使用チュートリアル
- `cargo run --example hybrid_best_practices` - 本番環境のベストプラクティス

**パフォーマンステスト**：
- `cargo run --example high_pressure_segqueue_test` - 高並行ストレステスト
- `cargo run --example performance_demo` - 基本パフォーマンスデモ

すべてのサンプルファイルには、Melange DBをすばやく理解して使用できるように、詳細なコードコメントと使用説明が含まれています。

#### 🧪 テストケース

```bash
# 高圧力並行テスト（12スレッド）
cargo run --example high_pressure_segqueue_test

# ハイブリッドマネージャーベストプラクティス
cargo run --example hybrid_best_practices

# ハイブリッドマネージャー使用ガイド
cargo run --example hybrid_manager_guide
```

#### 🔄 移行ガイド

**旧バージョン（v0.1.4以下）**:
```rust
// ❌ 非推奨 - EBR競合を引き起こす
let db = Arc::new(config.open()?);
// データベースへの直接的マルチスレッド操作はRefCell競合を引き起こす
```

**新バージョン（v0.2.0+）**:
```rust
// ✅ 推奨 - EBR競合なし
let manager = Arc::new(AtomicOperationsManager::new(Arc::new(config.open()?)));
// 統一ルーターを介した操作、完全にスレッドセーフ
```

📖 **詳細な移行ガイド**: 完全なアップグレード手順とトラブルシューティングガイドについては [docs/migration_guide_v0.2.0_ja.md](docs/migration_guide_v0.2.0_ja.md) を参照してください。

#### ⚡ パフォーマンス向上

- **並行安全性**: 無制限の並行スレッドをサポート
- **ゼロ競合**: EBR RefCell借用問題を完全に排除
- **自動永続化**: 原子操作完了後に自動的に永続化
- **データ一貫性**: 高並行下でのデータ完全性を保証

### 📦 効率的なメモリ管理
- **インクリメンタルシリアライゼーション**: I/Oオーバーヘッドを削減するシリアライゼーション戦略
- **スマートキャッシュ戦略**: 適応的キャッシュ置換アルゴリズム
- **メモリマッピング最適化**: 効率的なファイルマッピングメカニズム

## クイックスタート

### 📚 学習パス

**新規ユーザー推奨学習順序**：

1. **入門チュートリアル**: `cargo run --example hybrid_manager_guide`
   - ハイブリッドマネージャーの基本的な使用方法を学習
   - 原子操作とデータベース操作の統一インターフェースを理解
   - データ永続化とカウンターの使用をマスター

2. **ベストプラクティス**: `cargo run --example hybrid_best_practices`
   - 本番環境のベストプラクティスを学習
   - ユーザー管理、セッション処理などの実践的なシナリオをマスター
   - パフォーマンス最適化とエラーハンドリングを理解

3. **パフォーマンステスト**: `cargo run --example performance_demo`
   - Melange DBのパフォーマンス特性を理解
   - キャッシュ設定とフラッシュ戦略を学習
   - パフォーマンス監視方法をマスター

4. **高度な機能**: `cargo run --example rat_logger_demo`
   - ログシステムの統合を学習
   - デバッグと監視方法を理解

すべてのサンプルファイルは、Melange DBの様々な機能をすばやくマスターできるように、詳細な日本語コメントを含む完全に実行可能なプログラムです。

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
melange_db = "0.2.0"
```

## サンプル

### 🔥 利用可能なサンプル概覧

**ハイブリッドマネージャーアーキテクチャ（推奨）**：
- `cargo run --example hybrid_manager_guide` - 完全な使用チュートリアルとAPI紹介
- `cargo run --example hybrid_best_practices` - 本番環境のベストプラクティス
- `cargo run --example high_pressure_segqueue_test` - 12スレッド高並行ストレステスト

**パフォーマンステストと分析**：
- `cargo run --example performance_demo` - 基本パフォーマンスデモ
- `cargo run --example accurate_timing_demo` - 精密なタイミング分析（P50/P95/P99）
- `cargo run --example best_practices` - 従来APIのベストプラクティス

**システム統合**：
- `cargo run --example rat_logger_demo` - ログシステム統合
- `cargo run --example no_logger_test` - ログなし環境テスト

**プラットフォームパフォーマンステスト**：
- `cargo run --example macbook_air_m1_compression_none --features compression-none --release`
- `cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release`
- `cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release`

### 📊 パフォーマンスと機能テスト
- **パフォーマンスベンチマークテスト**: `cargo run --example performance_demo`
  - 基本パフォーマンスデモとスマートフラッシュ戦略のショーケース
  - 読み書きパフォーマンス統計とキャッシュヒット率分析を含む

- **精密なタイミング分析**: `cargo run --example accurate_timing_demo`
  - P50/P95/P99統計を含む詳細なパフォーマンス分析
  - 異なる操作タイプのレイテンシ分布を表示

- **ベストプラクティスデモ**: `cargo run --example best_practices`
  - 完全な本番環境使用例
  - ユーザーデータ管理、セッション処理、トランザクション操作などを含む

- **ログシステム統合**: `cargo run --example rat_logger_demo`
  - rat_logger高パフォーマンスログシステムの統合方法を表示
  - ログ設定とパフォーマンスデバッグ出力を実演

- **ロガーなしテスト**: `cargo run --example no_logger_test`
  - ロガーが初期化されていない場合の安全な動作を検証
  - ライブラリの後方互換性を表示

### 🖥️ プラットフォームパフォーマンステスト
- **M1 MacBook Airパフォーマンステスト**：
  ```bash
  # 無圧縮バージョン（最高パフォーマンス）
  cargo run --example macbook_air_m1_compression_none --features compression-none --release

  # LZ4圧縮バージョン（バランス型パフォーマンス）
  cargo run --example macbook_air_m1_compression_lz4 --features compression-lz4 --release

  # Zstd圧縮バージョン（高圧縮率）
  cargo run --example macbook_air_m1_compression_zstd --features compression-zstd --release
  ```

### ⚠️ 非推奨サンプル（v0.1.4以下）
- `simple_atomic_sequence` - 新統一アーキテクチャに移行済み
- `atomic_operations_test` - EBR競合問題あり、非推奨
- `atomic_mixed_operations` - 並行性制限あり、非推奨

### 🔄 移行提案

**最新のハイブリッドマネージャーアーキテクチャの使用を推奨**：
```bash
# 基本使用方法を学習
cargo run --example hybrid_manager_guide

# 本番環境リファレンス
cargo run --example hybrid_best_practices

# パフォーマンスストレステスト
cargo run --example high_pressure_segqueue_test
```

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
# ハイブリッドマネージャーアーキテクチャ（推奨）
cargo run --example hybrid_manager_guide
cargo run --example hybrid_best_practices

# パフォーマンステスト
cargo run --example performance_demo
cargo run --example accurate_timing_demo
cargo run --example high_pressure_segqueue_test

# システム統合
cargo run --example rat_logger_demo
cargo run --example no_logger_test

# プラットフォームパフォーマンステスト
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