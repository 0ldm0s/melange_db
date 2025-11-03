# Melange DB v0.2.0 移行ガイド

## 概要

Melange DB v0.2.0は**破壊的パフォーマンスアップグレード**で、全新しい原子操作統一アーキテクチャを導入し、高並行シナリオにおけるEBR (Epoch-Based Reclamation) RefCell競合を完全に解決しました。

これは破壊的アップグレードですが、移行プロセスを可能な限り簡単にするよう努めました。このガイドは旧バージョンからv0.2.0へ安全にアップグレードするヘルプとなります。

## 🚨 主要な変更点

### 解決された問題
- ✅ **EBR RefCell競合の完全排除**: マルチスレッド高並行操作で`RefCell already borrowed`パニックが発生しなくなりました
- ✅ **並行パフォーマンスの向上**: ワーカー間通信により大幅な並行パフォーマンス向上
- ✅ **データ一貫性保証**: 高並行下でのデータ完全性を確保

### API変更
- 🔄 **AtomicOperationsManager**: 新しい統一ルーター設計
- 🔄 **AtomicWorker**: 完全に独立した原子操作コンポーネントに再構築
- 🆕 **DatabaseWorker**: 新しい専用データベース操作ワーカー

## 移行ステップ

### ステップ1: 依存関係バージョンの更新

**Cargo.toml**:
```toml
[dependencies]
# 旧バージョン
melange_db = "0.1.5"

# 新バージョン
melange_db = "0.2.0"
```

### ステップ2: コード構造の更新

#### 旧バージョンコード (v0.1.5以下)
```rust
// ❌ このアプローチはEBR競合を引き起こします
use melange_db::{Db, Config};
use std::sync::Arc;
use std::thread;

fn main() -> anyhow::Result<()> {
    let config = Config::new().path("my_db");
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 直接のマルチスレッドデータベース操作 - EBR競合を引き起こします！
    let mut handles = vec![];
    for i in 0..4 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            // これらの操作は高並行下でRefCellパニックを引き起こします
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

#### 新バージョンコード (v0.2.0+)
```rust
// ✅ 推奨アプローチ - EBR競合なし
use melange_db::{Db, Config, atomic_operations_manager::AtomicOperationsManager};
use std::sync::Arc;
use std::thread;

fn main() -> anyhow::Result<()> {
    let config = Config::new().path("my_db");
    let db: Db<1024> = config.open()?;
    let db = Arc::new(db);

    // 統一ルーターを作成
    let manager = Arc::new(AtomicOperationsManager::new(db));

    // 統一ルーター経由のマルチスレッド操作 - 完全に安全！
    let mut handles = vec![];
    for i in 0..4 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            // 原子操作 - 自動永続化
            let counter = manager_clone.increment(format!("counter_{}", i), 1).unwrap();
            println!("スレッド{} カウンター: {}", i, counter);

            // データベース操作 - これも安全
            let key = format!("data:{}", i);
            let value = format!("スレッド{}からの値", i);
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

### ステップ3: 移行のテスト

移行が成功したことを確認するために以下のテストを実行してください：

```bash
# 基本統一アーキテクチャテスト
cargo run --example segqueue_unified_test

# 高圧力並行テスト（12スレッド）
cargo run --example high_pressure_segqueue_test

# 原子操作テスト
cargo run --example atomic_worker_test
```

## 一般的な移行シナリオ

### シナリオ1: 原子カウンター

**旧コード**:
```rust
// ❌ 旧アプローチ - EBR競合の可能性あり
let tree = db.open_tree("counters")?;
let new_value = tree.increment("user_counter")?;
```

**新コード**:
```rust
// ✅ 新アプローチ - 完全に安全
let new_value = manager.increment("user_counter".to_string(), 1)?;
```

### シナリオ2: ユーザーID割り当て

**旧コード**:
```rust
// ❌ 旧アプローチ
let user_id = tree.increment("user_id_allocator")?;
let user_key = format!("user:{}", user_id);
tree.insert(user_key.as_bytes(), user_data.as_bytes())?;
```

**新コード**:
```rust
// ✅ 新アプローチ
let user_id = manager.increment("user_id_allocator".to_string(), 1)?;
let user_key = format!("user:{}", user_id);
manager.insert(user_key.as_bytes(), user_data.as_bytes())?;
```

### シナリオ3: バッチ操作

**旧コード**:
```rust
// ❌ 旧アプローチ - 高並行下でクラッシュの可能性あり
for i in 0..1000 {
    let tree = db.open_tree("batch_data")?;
    tree.insert(&format!("key_{}", i), &format!("value_{}", i))?;
}
```

**新コード**:
```rust
// ✅ 新アプローチ - 完全に安全
for i in 0..1000 {
    let key = format!("key_{}", i);
    let value = format!("value_{}", i);
    manager.insert(key.as_bytes(), value.as_bytes())?;
}
```

## パフォーマンス比較

### 並行パフォーマンス

| メトリック | v0.1.5 | v0.2.0 | 改善 |
|-----------|--------|--------|------|
| 並行スレッドサポート | 2-4スレッド | 無制限 | ∞ |
| EBR競合 | 頻繁 | ゼロ競合 | 100% |
| データ一貫性 | 破損の可能性 | 完全保証 | 100% |

### テスト結果

**12スレッド高圧力テスト**:
- ✅ 285回の原子操作 (160 + 50 + 40 + 35回のページアクセス)
- ✅ 570件のデータベースレコード (300 + 150 + 120件)
- ✅ ゼロEBR競合
- ✅ 100%データ一貫性

## 互換性に関する注意

### データ互換性
- ✅ **完全に後方互換**: v0.1.5で作成されたデータベースファイルはv0.2.0で正常に読み取り可能
- ✅ **データ移行不要**: 既存データは変換操作不要

### API互換性
- ❌ **破壊的変更**: 原子操作APIは書き直しが必要
- ✅ **基本APIは変更なし**: 通常のデータベース読み書きAPIは変更なし
- ❌ **並行モード変更**: マルチスレッド並行アクセスパターンは更新が必要

## トラブルシューティング

### 問題1: コンパイルエラー

**エラー**: `cannot find function AtomicOperationsManager`

**解決策**: バージョンが正しく更新されていることを確認：
```bash
cargo clean
cargo update
```

### 問題2: ランタイムエラー

**エラー**: 原子カウンターデータが見つからない

**解決策**: プリヒーティング機能を使用して古いデータをロード：
```rust
// 既存の原子カウンターをプリロード
let loaded_count = manager.preload_counters()?;
println!("{}個のカウンターをプリロードしました", loaded_count);
```

### 問題3: パフォーマンスの問題

**症状**: アップグレード後にパフォーマンスが低下

**解決策**: 統一ルーターを正しく使用しているか確認：
```rust
// ✅ 正しい - すべての操作をmanager経由で
let value = manager.increment("counter".to_string(), 1)?;
manager.insert(key, value)?;

// ❌ 間違い - 新旧APIの混合使用
let db = manager.database_worker().db(); // これはしないでください！
```

## ロールバック計画

アップグレード中に問題が発生した場合、一時的に旧バージョンにロールバックできます：

```toml
# 一時的ロールバック
melange_db = "0.1.5"
```

**注意**: ロールバックする前にデータベースファイルをバックアップしてください！

## ヘルプの取得

移行中に問題が発生した場合：

1. **サンプルコードを確認**: `examples/`ディレクトリの完全なサンプル
2. **テストを実行**: 提供されたテストケースで機能を検証
3. **ログを確認**: 詳細なログを有効にして特定のエラーメッセージを確認

## まとめ

v0.2.0への移行にはいくつかのコード変更が必要ですが、利点は非常に大きいです：

- 🚀 **ゼロ並行競合**: EBR問題を完全に解決
- 📈 **無制限の並行性**: 無制限の並行スレッドをサポート
- 🔒 **データ一貫性**: 高並行下でのデータ完全性を保証
- ⚡ **パフォーマンス向上**: 全体的な並行パフォーマンスが大幅に改善

このガイドの手順に従って、安全かつ円滑にアップグレードを完了してください。

---

**アップグレード後には完全なテストスイートの実行を強く推奨**:
```bash
cargo test
cargo run --example segqueue_unified_test
cargo run --example high_pressure_segqueue_test
```

ご利用ありがとうございます！🎉