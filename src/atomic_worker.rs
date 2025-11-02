//! 原子操作Worker
//!
//! 使用SegQueue + Worker线程实现高性能原子计数器操作
//! 避免直接并发操作持久化层，提高并发性能

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{debug_log, trace_log, warn_log, error_log, info_log};
use crate::db::Db;

/// 原子操作类型
#[derive(Debug, Clone)]
pub enum AtomicOperation {
    /// 原子递增
    Increment {
        counter_name: String,
        delta: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 获取计数器值
    Get {
        counter_name: String,
        response_tx: std::sync::mpsc::Sender<io::Result<Option<u64>>>,
    },
    /// 重置计数器
    Reset {
        counter_name: String,
        new_value: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<()>>,
    },
}

/// 原子操作Worker
///
/// 完全独立的原子计数器管理组件，不持有任何数据库引用
pub struct AtomicWorker {
    /// 内存中的原子计数器 (使用DashMap提供高性能并发访问)
    counters: Arc<DashMap<String, Arc<AtomicU64>>>,

    /// 操作队列 (无锁并发队列)
    operation_queue: Arc<SegQueue<AtomicOperation>>,

    /// Worker句柄
    worker_handle: Option<thread::JoinHandle<()>>,

    /// 关闭信号
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl AtomicWorker {
    /// 创建新的原子操作Worker
    ///
    /// 完全独立，不持有任何数据库引用
    pub fn new(_db: Option<Arc<Db<1024>>>) -> Self {
        let counters = Arc::new(DashMap::new());
        let operation_queue = Arc::new(SegQueue::new());
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

        let worker_counters = counters.clone();
        let worker_queue = operation_queue.clone();

        let worker_handle = thread::spawn(move || {
            debug_log!("原子操作Worker线程启动");
            Self::worker_loop(worker_counters, worker_queue, shutdown_rx);
            debug_log!("原子操作Worker线程退出");
        });

        Self {
            counters,
            operation_queue,
            worker_handle: Some(worker_handle),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Worker主循环
    fn worker_loop(
        counters: Arc<DashMap<String, Arc<AtomicU64>>>,
        operation_queue: Arc<SegQueue<AtomicOperation>>,
        shutdown_rx: std::sync::mpsc::Receiver<()>,
    ) {
        loop {
            // 检查关闭信号
            match shutdown_rx.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    debug_log!("收到关闭信号，Worker退出");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 继续处理操作
                }
            }

            // 处理操作队列
            if let Some(operation) = operation_queue.pop() {
                Self::handle_operation(&counters, operation);
            } else {
                // 队列为空，短暂休眠避免CPU占用过高
                thread::yield_now();
            }
        }
    }

    /// 处理单个原子操作
    fn handle_operation(
        counters: &DashMap<String, Arc<AtomicU64>>,
        operation: AtomicOperation,
    ) {
        match operation {
            AtomicOperation::Increment { counter_name, delta, response_tx } => {
                let result = Self::handle_increment(counters, &counter_name, delta);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Get { counter_name, response_tx } => {
                let result = Self::handle_get(counters, &counter_name);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Reset { counter_name, new_value, response_tx } => {
                let result = Self::handle_reset(counters, &counter_name, new_value);
                let _ = response_tx.send(result);
            }
        }
    }

    /// 处理原子递增操作
    fn handle_increment(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        delta: u64,
    ) -> io::Result<u64> {
        trace_log!("处理原子递增: {} + {}", counter_name, delta);

        // 获取或创建原子计数器
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        // 执行原子递增（纯内存操作）
        let new_value = counter.fetch_add(delta, Ordering::SeqCst) + delta;

        trace_log!("原子递增完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 处理获取计数器操作
    fn handle_get(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
    ) -> io::Result<Option<u64>> {
        trace_log!("处理获取计数器: {}", counter_name);

        if let Some(counter) = counters.get(counter_name) {
            let value = counter.load(Ordering::SeqCst);
            trace_log!("获取计数器完成: {} = {}", counter_name, value);
            Ok(Some(value))
        } else {
            trace_log!("计数器不存在: {}", counter_name);
            Ok(None)
        }
    }

    /// 处理重置计数器操作
    fn handle_reset(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        new_value: u64,
    ) -> io::Result<()> {
        trace_log!("处理重置计数器: {} = {}", counter_name, new_value);

        // 更新内存中的原子计数器（纯内存操作）
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        counter.store(new_value, Ordering::SeqCst);

        trace_log!("重置计数器完成: {} = {}", counter_name, new_value);
        Ok(())
    }

    /// 提交原子递增操作
    pub fn increment(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Increment {
            counter_name,
            delta,
            response_tx,
        };

        self.operation_queue.push(operation);

        // 等待Worker处理结果
        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交获取计数器操作
    pub fn get(&self, counter_name: String) -> io::Result<Option<u64>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Get {
            counter_name,
            response_tx,
        };

        self.operation_queue.push(operation);

        // 等待Worker处理结果
        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交重置计数器操作
    pub fn reset(&self, counter_name: String, new_value: u64) -> io::Result<()> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Reset {
            counter_name,
            new_value,
            response_tx,
        };

        self.operation_queue.push(operation);

        // 等待Worker处理结果
        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 加载单个计数器（供Manager调用）
    pub fn load_counter(&self, counter_name: String, value: u64) {
        trace_log!("加载计数器: {} = {}", counter_name, value);
        let counter = Arc::new(AtomicU64::new(value));
        self.counters.insert(counter_name, counter);
    }

    /// 获取所有计数器名称（供调试使用）
    pub fn get_counter_names(&self) -> Vec<String> {
        self.counters.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Drop for AtomicWorker {
    fn drop(&mut self) {
        debug_log!("开始关闭原子操作Worker");

        // 发送关闭信号
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // 等待Worker线程退出
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        debug_log!("原子操作Worker已关闭");
    }
}

// 重新导出io::Result
use std::io;