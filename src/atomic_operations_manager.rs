//! 原子操作统一路由器
//!
//! 作为纯路由器，不直接操作任何数据结构
//! 负责将操作路由到相应的Worker

use std::sync::Arc;
use std::io;

use crate::{debug_log, trace_log, warn_log, error_log, info_log, InlineArray};
use crate::db::Db;
use super::atomic_worker::AtomicWorker;
use super::database_worker::DatabaseWorker;

/// 原子操作统一路由器
///
/// 纯路由器，负责将操作分发到相应的Worker
pub struct AtomicOperationsManager {
    /// 原子操作Worker
    atomic_worker: Arc<AtomicWorker>,

    /// 数据库操作Worker
    database_worker: Arc<DatabaseWorker>,
}

impl AtomicOperationsManager {
    /// 创建新的原子操作统一路由器
    ///
    /// # Arguments
    /// * `db` - 数据库实例引用
    pub fn new(db: Arc<Db<1024>>) -> Self {
        debug_log!("创建原子操作统一路由器");

        // 创建数据库操作Worker
        let database_worker = Arc::new(DatabaseWorker::new(db.clone()));

        // 创建原子操作Worker，传入数据库Worker的队列引用
        let atomic_worker = Arc::new(AtomicWorker::new(Some(database_worker.operation_queue().clone())));

        Self {
            atomic_worker,
            database_worker,
        }
    }

    /// 原子递增操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `delta` - 递增量
    pub fn increment(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        trace_log!("路由原子递增操作: {} + {}", counter_name, delta);

        // 路由到原子操作Worker（AtomicWorker会自动向DatabaseWorker发送持久化指令）
        self.atomic_worker.increment(counter_name, delta)
    }

    /// 原子递减操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `delta` - 递减量
    pub fn decrement(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        trace_log!("路由原子递减操作: {} - {}", counter_name, delta);
        self.atomic_worker.decrement(counter_name, delta)
    }

    /// 原子乘法操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `factor` - 乘法因子
    pub fn multiply(&self, counter_name: String, factor: u64) -> io::Result<u64> {
        trace_log!("路由原子乘法操作: {} * {}", counter_name, factor);
        self.atomic_worker.multiply(counter_name, factor)
    }

    /// 原子除法操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `divisor` - 除数
    pub fn divide(&self, counter_name: String, divisor: u64) -> io::Result<u64> {
        trace_log!("路由原子除法操作: {} / {}", counter_name, divisor);
        self.atomic_worker.divide(counter_name, divisor)
    }

    /// 原子百分比操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `percentage` - 百分比值 (0-100)
    pub fn percentage(&self, counter_name: String, percentage: u64) -> io::Result<u64> {
        trace_log!("路由原子百分比操作: {} * {}%", counter_name, percentage);
        self.atomic_worker.percentage(counter_name, percentage)
    }

    /// 原子比较和交换操作
    ///
    /// # Arguments
    /// * `counter_name` - 计数器名称
    /// * `expected` - 期望的当前值
    /// * `new_value` - 要设置的新值
    pub fn compare_and_swap(&self, counter_name: String, expected: u64, new_value: u64) -> io::Result<bool> {
        trace_log!("路由原子比较和交换操作: {} (expected: {}, new: {})", counter_name, expected, new_value);
        self.atomic_worker.compare_and_swap(counter_name, expected, new_value)
    }

    /// 获取计数器值
    pub fn get(&self, counter_name: String) -> io::Result<Option<u64>> {
        trace_log!("路由获取计数器操作: {}", counter_name);
        self.atomic_worker.get(counter_name)
    }

    /// 重置计数器
    pub fn reset(&self, counter_name: String, new_value: u64) -> io::Result<()> {
        trace_log!("路由重置计数器操作: {} = {}", counter_name, new_value);
        self.atomic_worker.reset(counter_name, new_value)
    }

    /// 预热原子计数器（从持久层加载）
    pub fn preload_counters(&self) -> io::Result<usize> {
        debug_log!("路由预热计数器操作");

        // 路由到数据库Worker
        let counters = self.database_worker.preload_counters()?;
        let count = counters.len();

        // 加载到原子操作Worker
        for (name, value) in counters {
            self.atomic_worker.load_counter(name.clone(), value);
            trace_log!("预热计数器: {} = {}", name, value);
        }

        Ok(count)
    }

    /// 执行数据库插入操作
    pub fn insert(&self, key: &[u8], value: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("路由数据库插入操作: {:?}", key);
        self.database_worker.insert(key.to_vec(), value.to_vec())
    }

    /// 执行数据库获取操作
    pub fn get_data(&self, key: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("路由数据库获取操作: {:?}", key);
        self.database_worker.get(key.to_vec())
    }

    /// 扫描前缀操作
    pub fn scan_prefix(&self, prefix: &[u8]) -> io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        trace_log!("路由扫描前缀操作: {:?}", prefix);
        self.database_worker.scan_prefix(prefix.to_vec())
    }

    /// 执行数据库删除操作
    pub fn remove(&self, key: &[u8]) -> io::Result<Option<InlineArray>> {
        trace_log!("路由数据库删除操作: {:?}", key);
        self.database_worker.remove(key.to_vec())
    }

    /// 检查键是否存在
    pub fn contains_key(&self, key: &[u8]) -> io::Result<bool> {
        trace_log!("路由检查键存在操作: {:?}", key);
        self.database_worker.contains_key(key.to_vec())
    }

    /// 清空所有数据
    pub fn clear(&self) -> io::Result<()> {
        trace_log!("路由清空数据库操作");
        self.database_worker.clear()
    }

    /// 获取键值对总数
    pub fn len(&self) -> io::Result<usize> {
        trace_log!("路由获取键值对总数操作");
        self.database_worker.len()
    }

    /// 检查数据库是否为空
    pub fn is_empty(&self) -> io::Result<bool> {
        trace_log!("路由检查数据库是否为空操作");
        self.database_worker.is_empty()
    }

    /// 获取第一个键值对
    pub fn first(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        trace_log!("路由获取第一个键值对操作");
        self.database_worker.first()
    }

    /// 获取最后一个键值对
    pub fn last(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        trace_log!("路由获取最后一个键值对操作");
        self.database_worker.last()
    }

    /// 获取原子操作Worker引用（用于高级操作）
    pub fn atomic_worker(&self) -> &AtomicWorker {
        &self.atomic_worker
    }

    /// 获取数据库Worker引用（用于高级操作）
    pub fn database_worker(&self) -> &DatabaseWorker {
        &self.database_worker
    }
}