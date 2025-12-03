//! 混合操作管理器
//!
//! 结合直接访问和原子操作的优点：
//! - 普通数据库操作：直接访问，零额外开销
//! - 原子计数器操作：通过统一架构，保证并发安全

use std::sync::Arc;
use std::io;

use crate::{debug_log, trace_log, warn_log, error_log, info_log, InlineArray};
use crate::db::Db;
use super::atomic_worker::AtomicWorker;
use super::database_worker::DatabaseWorker;

/// 混合操作管理器
///
/// 智能选择最优路径：
/// - 原子操作 → AtomicWorker（保证并发安全）
/// - 普通操作 → 直接访问（零开销）
pub struct HybridOperationsManager {
    /// 数据库实例（用于直接访问）
    db: Arc<Db<1024>>,

    /// 原子操作Worker（仅用于原子计数器）
    atomic_worker: Arc<AtomicWorker>,

    /// 数据库操作Worker（仅用于特殊场景）
    database_worker: Option<Arc<DatabaseWorker>>,
}

impl HybridOperationsManager {
    /// 创建新的混合操作管理器
    pub fn new(db: Arc<Db<1024>>) -> Self {
        debug_log!("创建混合操作管理器");

        // 创建原子操作Worker（不需要数据库Worker队列）
        let atomic_worker = Arc::new(AtomicWorker::new(None));

        Self {
            db,
            atomic_worker,
            database_worker: None,
        }
    }

    /// 创建带数据库Worker的管理器（特殊场景使用）
    pub fn new_with_db_worker(db: Arc<Db<1024>>) -> Self {
        debug_log!("创建混合操作管理器（含数据库Worker）");

        let database_worker = Arc::new(DatabaseWorker::new(db.clone()));
        let atomic_worker = Arc::new(AtomicWorker::new(Some(database_worker.operation_queue().clone())));

        Self {
            db,
            atomic_worker,
            database_worker: Some(database_worker),
        }
    }

    // ========== 原子操作：通过AtomicWorker ==========

    /// 原子递增操作
    pub fn increment(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        trace_log!("执行原子递增: {} + {}", counter_name, delta);
        self.atomic_worker.increment(counter_name, delta)
    }

    /// 原子递减操作
    pub fn decrement(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        trace_log!("执行原子递减: {} - {}", counter_name, delta);
        self.atomic_worker.decrement(counter_name, delta)
    }

    /// 原子乘法操作
    pub fn multiply(&self, counter_name: String, factor: u64) -> io::Result<u64> {
        trace_log!("执行原子乘法: {} * {}", counter_name, factor);
        self.atomic_worker.multiply(counter_name, factor)
    }

    /// 原子除法操作
    pub fn divide(&self, counter_name: String, divisor: u64) -> io::Result<u64> {
        trace_log!("执行原子除法: {} / {}", counter_name, divisor);
        self.atomic_worker.divide(counter_name, divisor)
    }

    /// 原子百分比操作
    pub fn percentage(&self, counter_name: String, percentage: u64) -> io::Result<u64> {
        trace_log!("执行原子百分比: {} * {}%", counter_name, percentage);
        self.atomic_worker.percentage(counter_name, percentage)
    }

    /// 原子比较和交换操作
    pub fn compare_and_swap(&self, counter_name: String, expected: u64, new_value: u64) -> io::Result<bool> {
        trace_log!("执行原子比较和交换: {} (expected: {}, new: {})", counter_name, expected, new_value);
        self.atomic_worker.compare_and_swap(counter_name, expected, new_value)
    }

    /// 获取计数器值
    pub fn get(&self, counter_name: String) -> io::Result<Option<u64>> {
        trace_log!("执行获取计数器: {}", counter_name);
        self.atomic_worker.get(counter_name)
    }

    /// 重置计数器
    pub fn reset(&self, counter_name: String, new_value: u64) -> io::Result<()> {
        trace_log!("执行重置计数器: {} = {}", counter_name, new_value);
        self.atomic_worker.reset(counter_name, new_value)
    }

    /// 预热原子计数器
    pub fn preload_counters(&self) -> io::Result<usize> {
        debug_log!("预热原子计数器");

        // 直接从数据库加载计数器
        let mut counters = Vec::new();
        let prefix = b"__atomic_counter__:";

        for item_res in self.db.scan_prefix(prefix) {
            if let Ok((key_bytes, value_bytes)) = item_res {
                let key_bytes = &*key_bytes;
                let value_bytes = &*value_bytes;

                if let Ok(key_str) = std::str::from_utf8(key_bytes) {
                    if let Some(counter_name) = key_str.strip_prefix("__atomic_counter__:") {
                        if value_bytes.len() >= 8 {
                            let mut arr = [0u8; 8];
                            arr.copy_from_slice(&value_bytes[..8]);
                            let value = u64::from_le_bytes(arr);
                            counters.push((counter_name.to_string(), value));
                        }
                    }
                }
            }
        }

        let count = counters.len();

        // 加载到原子操作Worker
        for (name, value) in counters {
            self.atomic_worker.load_counter(name.clone(), value);
            trace_log!("预热计数器: {} = {}", name, value);
        }

        Ok(count)
    }

    // ========== 普通数据库操作：直接访问 ==========

    /// 执行数据库插入操作（直接访问）
    pub fn insert(&self, key: &[u8], value: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("直接数据库插入: {:?}", key);

        // 检查是否需要通过DatabaseWorker（特殊场景）
        if let Some(db_worker) = &self.database_worker {
            // 特殊场景：通过DatabaseWorker
            db_worker.insert(key.to_vec(), value.to_vec())
        } else {
            // 默认场景：直接访问，零开销
            self.db.insert(key, value)
        }
    }

    /// 执行数据库获取操作（直接访问）
    pub fn get_data(&self, key: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("直接数据库获取: {:?}", key);

        if let Some(db_worker) = &self.database_worker {
            db_worker.get(key.to_vec())
        } else {
            self.db.get(key)
        }
    }

    /// 扫描前缀操作（直接访问）
    pub fn scan_prefix(&self, prefix: &[u8]) -> io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        trace_log!("直接扫描前缀: {:?}", prefix);

        let result = self.db.scan_prefix(prefix)
            .collect::<io::Result<Vec<_>>>()
            .map(|items| {
                items.into_iter()
                    .map(|(key, value)| (key.to_vec(), value.to_vec()))
                    .collect()
            });

        result
    }

    /// 执行数据库删除操作（直接访问）
    pub fn remove(&self, key: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("直接数据库删除: {:?}", key);

        if let Some(db_worker) = &self.database_worker {
            db_worker.remove(key.to_vec())
        } else {
            self.db.remove(key)
        }
    }

    /// 检查键是否存在（直接访问）
    pub fn contains_key(&self, key: &[u8]) -> io::Result<bool> {
        trace_log!("直接检查键存在: {:?}", key);

        if let Some(db_worker) = &self.database_worker {
            db_worker.contains_key(key.to_vec())
        } else {
            self.db.contains_key(key)
        }
    }

    /// 清空所有数据（直接访问）
    pub fn clear(&self) -> io::Result<()> {
        trace_log!("直接清空数据库");

        if let Some(db_worker) = &self.database_worker {
            db_worker.clear()
        } else {
            self.db.clear()
        }
    }

    /// 获取键值对总数（直接访问）
    pub fn len(&self) -> io::Result<usize> {
        trace_log!("直接获取键值对总数");

        if let Some(db_worker) = &self.database_worker {
            db_worker.len()
        } else {
            self.db.len()
        }
    }

    /// 检查数据库是否为空（直接访问）
    pub fn is_empty(&self) -> io::Result<bool> {
        trace_log!("直接检查数据库是否为空");

        if let Some(db_worker) = &self.database_worker {
            db_worker.is_empty()
        } else {
            self.db.is_empty()
        }
    }

    /// 获取第一个键值对（直接访问）
    pub fn first(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        trace_log!("直接获取第一个键值对");

        if let Some(db_worker) = &self.database_worker {
            db_worker.first()
        } else {
            self.db.first()
        }
    }

    /// 获取最后一个键值对（直接访问）
    pub fn last(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        trace_log!("直接获取最后一个键值对");

        if let Some(db_worker) = &self.database_worker {
            db_worker.last()
        } else {
            self.db.last()
        }
    }

    /// 启用数据库Worker模式（特殊场景）
    pub fn enable_database_worker_mode(&mut self) {
        if self.database_worker.is_none() {
            debug_log!("启用数据库Worker模式");
            self.database_worker = Some(Arc::new(DatabaseWorker::new(self.db.clone())));

            // 重新创建AtomicWorker，连接到DatabaseWorker
            self.atomic_worker = Arc::new(AtomicWorker::new(
                Some(self.database_worker.as_ref().unwrap().operation_queue().clone())
            ));
        }
    }

    /// 禁用数据库Worker模式（默认高性能模式）
    pub fn disable_database_worker_mode(&mut self) {
        if self.database_worker.is_some() {
            debug_log!("禁用数据库Worker模式，切换到直接访问");
            self.database_worker = None;

            // 重新创建AtomicWorker，不连接DatabaseWorker
            self.atomic_worker = Arc::new(AtomicWorker::new(None));
        }
    }

    /// 获取原子操作Worker引用（用于高级操作）
    pub fn atomic_worker(&self) -> &AtomicWorker {
        &self.atomic_worker
    }

    /// 获取数据库实例引用（用于高级操作）
    pub fn db(&self) -> &Db<1024> {
        &self.db
    }
}