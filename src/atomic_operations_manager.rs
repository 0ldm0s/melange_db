//! 原子操作管理器
//!
//! 作为统一入口，持有数据库引用和独立的原子操作组件
//! 负责操作分发和持久化处理

use std::sync::Arc;
use std::io;

use crate::{debug_log, trace_log, warn_log, error_log, info_log, InlineArray};
use crate::db::Db;
use super::atomic_worker::AtomicWorker;

/// 原子操作管理器
///
/// 作为统一入口，协调原子操作和数据库操作
pub struct AtomicOperationsManager {
    /// 数据库引用（用于持久化和常规操作）
    db: Arc<Db<1024>>,

    /// 独立的原子操作Worker（不持有Db引用）
    atomic_worker: Arc<AtomicWorker>,
}

impl AtomicOperationsManager {
    /// 创建新的原子操作管理器
    ///
    /// # Arguments
    /// * `db` - 数据库实例引用
    pub fn new(db: Arc<Db<1024>>) -> Self {
        debug_log!("创建原子操作管理器");

        // 创建独立的原子操作Worker（传入None作为db引用）
        let atomic_worker = Arc::new(AtomicWorker::new(None));

        Self {
            db,
            atomic_worker,
        }
    }

    /// 原子递增操作（仅内存，不持久化）
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `delta` - 递增量
    pub fn increment(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        trace_log!("执行原子递增: {} + {}", counter_name, delta);

        // 通过独立的AtomicWorker执行原子递增（纯内存操作）
        let new_value = self.atomic_worker.increment(counter_name.clone(), delta)?;

        trace_log!("原子递增完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 手动持久化指定计数器
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    pub fn persist_counter(&self, counter_name: &str) -> io::Result<()> {
        trace_log!("持久化计数器: {}", counter_name);

        if let Some(value) = self.atomic_worker.get(counter_name.to_string())? {
            let key = format!("__atomic_counter__:{}", counter_name);
            self.db.insert(key.as_bytes(), value.to_le_bytes())?;
            trace_log!("持久化完成: {} = {}", counter_name, value);
        }

        Ok(())
    }

    /// 手动持久化所有计数器
    pub fn persist_all_counters(&self) -> io::Result<usize> {
        debug_log!("持久化所有计数器");
        let counter_names = self.atomic_worker.get_counter_names();
        let mut persisted_count = 0;

        for counter_name in counter_names {
            self.persist_counter(&counter_name)?;
            persisted_count += 1;
        }

        debug_log!("持久化完成，共处理 {} 个计数器", persisted_count);
        Ok(persisted_count)
    }

    /// 获取计数器值
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    pub fn get(&self, counter_name: String) -> io::Result<Option<u64>> {
        trace_log!("获取计数器值: {}", counter_name);

        // 直接从独立AtomicWorker获取
        self.atomic_worker.get(counter_name)
    }

    /// 重置计数器（仅内存，不持久化）
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `new_value` - 新值
    pub fn reset(&self, counter_name: String, new_value: u64) -> io::Result<()> {
        trace_log!("重置计数器: {} = {}", counter_name, new_value);

        // 通过独立的AtomicWorker重置（纯内存操作）
        self.atomic_worker.reset(counter_name.clone(), new_value)?;

        trace_log!("重置计数器完成: {} = {}", counter_name, new_value);
        Ok(())
    }

    /// 预热原子计数器（从持久层加载）
    pub fn preload_counters(&self) -> io::Result<usize> {
        debug_log!("开始预热原子计数器...");

        // 扫描所有原子计数器键
        let prefix = b"__atomic_counter__:";
        let mut loaded_count = 0;

        for item_res in self.db.scan_prefix(prefix) {
            let (key_bytes, value_bytes) = item_res?;
            let key_bytes = &*key_bytes;
            let value_bytes = &*value_bytes;

            // 解析计数器名称
            if let Ok(key_str) = std::str::from_utf8(key_bytes) {
                if let Some(counter_name) = key_str.strip_prefix("__atomic_counter__:") {
                    // 解析计数器值
                    if value_bytes.len() >= 8 {
                        let mut arr = [0u8; 8];
                        arr.copy_from_slice(&value_bytes[..8]);
                        let value = u64::from_le_bytes(arr);

                        // 加载到独立的AtomicWorker
                        self.atomic_worker.load_counter(counter_name.to_string(), value);
                        loaded_count += 1;

                        trace_log!("预热计数器: {} = {}", counter_name, value);
                    }
                }
            }
        }

        debug_log!("预热完成，加载了 {} 个原子计数器", loaded_count);
        Ok(loaded_count)
    }

    /// 执行常规数据库操作（插入）
    ///
    /// # Arguments
    /// * `key` - 键
    /// * `value` - 值
    pub fn insert(&self, key: &[u8], value: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("执行常规数据库插入: {:?}", key);
        self.db.insert(key, value)
    }

    /// 执行常规数据库操作（获取）
    ///
    /// # Arguments
    /// * `key` - 键
    pub fn get_data(&self, key: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("执行常规数据库获取: {:?}", key);
        self.db.get(key)
    }

    /// 获取数据库引用（用于复杂操作）
    pub fn db(&self) -> &Db<1024> {
        &self.db
    }

    /// 获取原子操作Worker引用（用于高级原子操作）
    pub fn atomic_worker(&self) -> &AtomicWorker {
        &self.atomic_worker
    }
}