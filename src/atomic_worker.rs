//! 原子操作Worker
//!
//! 使用SegQueue + Worker线程实现高性能原子计数器操作
//! 避免直接并发操作持久化层，提高并发性能

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{debug_log, trace_log, warn_log, error_log, info_log};
use super::database_worker::DatabaseOperation;

/// 原子操作类型
#[derive(Debug, Clone)]
pub(crate) enum AtomicOperation {
    /// 原子递增
    Increment {
        counter_name: String,
        delta: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 原子递减
    Decrement {
        counter_name: String,
        delta: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 原子乘法
    Multiply {
        counter_name: String,
        factor: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 原子除法
    Divide {
        counter_name: String,
        divisor: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 原子百分比计算
    Percentage {
        counter_name: String,
        percentage: u64, // 0-100的百分比值
        response_tx: std::sync::mpsc::Sender<io::Result<u64>>,
    },
    /// 原子比较和交换
    CompareAndSwap {
        counter_name: String,
        expected: u64,
        new_value: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<bool>>,
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
/// 专门处理原子操作，完成后自动向DatabaseWorker发送持久化指令
/// 内部实现细节，不应直接对外暴露
pub(crate) struct AtomicWorker {
    /// 内存中的原子计数器 (使用DashMap提供高性能并发访问)
    counters: Arc<DashMap<String, Arc<AtomicU64>>>,

    /// 操作队列 (无锁并发队列)
    operation_queue: Arc<SegQueue<AtomicOperation>>,

    /// Worker句柄
    worker_handle: Option<thread::JoinHandle<()>>,

    /// 关闭信号
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,

    /// 数据库Worker操作队列引用 (用于发送持久化指令)
    db_queue: Option<Arc<SegQueue<DatabaseOperation>>>,
}

impl AtomicWorker {
    /// 创建新的原子操作Worker
    ///
    /// # Arguments
    /// * `db_queue` - 数据库Worker操作队列引用，用于发送持久化指令
    pub(crate) fn new(db_queue: Option<Arc<SegQueue<DatabaseOperation>>>) -> Self {
        let counters = Arc::new(DashMap::new());
        let operation_queue = Arc::new(SegQueue::new());
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

        let worker_counters = counters.clone();
        let worker_queue = operation_queue.clone();
        let worker_db_queue = db_queue.clone();

        let worker_handle = thread::spawn(move || {
            debug_log!("原子操作Worker线程启动");
            Self::worker_loop(worker_counters, worker_queue, worker_db_queue, shutdown_rx);
            debug_log!("原子操作Worker线程退出");
        });

        Self {
            counters,
            operation_queue,
            worker_handle: Some(worker_handle),
            shutdown_tx: Some(shutdown_tx),
            db_queue,
        }
    }

    /// Worker主循环
    fn worker_loop(
        counters: Arc<DashMap<String, Arc<AtomicU64>>>,
        operation_queue: Arc<SegQueue<AtomicOperation>>,
        db_queue: Option<Arc<SegQueue<DatabaseOperation>>>,
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
                Self::handle_operation(&counters, operation, &db_queue);
            } else {
                // 队列为空，短暂休眠避免CPU占用过高
                thread::sleep(Duration::from_micros(500)); // 0.5ms休眠
            }
        }
    }

    /// 处理单个原子操作
    fn handle_operation(
        counters: &DashMap<String, Arc<AtomicU64>>,
        operation: AtomicOperation,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) {
        match operation {
            AtomicOperation::Increment { counter_name, delta, response_tx } => {
                let result = Self::handle_increment(counters, &counter_name, delta, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Decrement { counter_name, delta, response_tx } => {
                let result = Self::handle_decrement(counters, &counter_name, delta, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Multiply { counter_name, factor, response_tx } => {
                let result = Self::handle_multiply(counters, &counter_name, factor, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Divide { counter_name, divisor, response_tx } => {
                let result = Self::handle_divide(counters, &counter_name, divisor, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Percentage { counter_name, percentage, response_tx } => {
                let result = Self::handle_percentage(counters, &counter_name, percentage, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::CompareAndSwap { counter_name, expected, new_value, response_tx } => {
                let result = Self::handle_compare_and_swap(counters, &counter_name, expected, new_value, db_queue);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Get { counter_name, response_tx } => {
                let result = Self::handle_get(counters, &counter_name);
                let _ = response_tx.send(result);
            }
            AtomicOperation::Reset { counter_name, new_value, response_tx } => {
                let result = Self::handle_reset(counters, &counter_name, new_value, db_queue);
                let _ = response_tx.send(result);
            }
        }
    }

    /// 处理原子递增操作
    fn handle_increment(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        delta: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<u64> {
        trace_log!("处理原子递增: {} + {}", counter_name, delta);

        // 获取或创建原子计数器
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        // 执行原子递增（纯内存操作）
        let new_value = counter.fetch_add(delta, Ordering::SeqCst) + delta;

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0, // 不需要响应，直接丢弃
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

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

    /// 处理原子递减操作
    fn handle_decrement(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        delta: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<u64> {
        trace_log!("处理原子递减: {} - {}", counter_name, delta);

        // 获取或创建原子计数器
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        // 执行原子递减（防止下溢）
        let current_value = counter.load(Ordering::SeqCst);
        let new_value = if current_value >= delta {
            counter.fetch_sub(delta, Ordering::SeqCst) - delta
        } else {
            // 如果当前值小于递减值，设为0
            counter.store(0, Ordering::SeqCst);
            0
        };

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0,
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

        trace_log!("原子递减完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 处理原子乘法操作
    fn handle_multiply(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        factor: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<u64> {
        trace_log!("处理原子乘法: {} * {}", counter_name, factor);

        // 检查乘法溢出
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        let current_value = counter.load(Ordering::SeqCst);
        let new_value = match current_value.checked_mul(factor) {
            Some(result) => result,
            None => {
                warn_log!("乘法溢出: {} * {}, 设为u64::MAX", current_value, factor);
                u64::MAX
            }
        };

        counter.store(new_value, Ordering::SeqCst);

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0,
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

        trace_log!("原子乘法完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 处理原子除法操作
    fn handle_divide(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        divisor: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<u64> {
        trace_log!("处理原子除法: {} / {}", counter_name, divisor);

        if divisor == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "除数不能为零"));
        }

        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        let current_value = counter.load(Ordering::SeqCst);
        let new_value = current_value / divisor;

        counter.store(new_value, Ordering::SeqCst);

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0,
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

        trace_log!("原子除法完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 处理原子百分比操作
    fn handle_percentage(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        percentage: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<u64> {
        trace_log!("处理原子百分比: {} * {}%", counter_name, percentage);

        if percentage > 100 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "百分比值不能超过100"));
        }

        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        let current_value = counter.load(Ordering::SeqCst);
        let new_value = (current_value * percentage) / 100;

        counter.store(new_value, Ordering::SeqCst);

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0,
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

        trace_log!("原子百分比完成: {} = {}", counter_name, new_value);
        Ok(new_value)
    }

    /// 处理原子比较和交换操作
    fn handle_compare_and_swap(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        expected: u64,
        new_value: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<bool> {
        trace_log!("处理原子比较和交换: {} (expected: {}, new: {})", counter_name, expected, new_value);

        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        // 使用原子比较和交换操作
        let result = counter.compare_exchange_weak(
            expected,
            new_value,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_ok();

        if result {
            // CAS成功，发送持久化指令
            if let Some(db_queue) = db_queue {
                let persist_op = DatabaseOperation::PersistCounter {
                    counter_name: counter_name.to_string(),
                    value: new_value,
                    response_tx: std::sync::mpsc::channel().0,
                };
                db_queue.push(persist_op);
                trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
            }
            trace_log!("原子比较和交换成功: {} = {}", counter_name, new_value);
        } else {
            trace_log!("原子比较和交换失败: {} 值不匹配", counter_name);
        }

        Ok(result)
    }

    /// 处理重置计数器操作
    fn handle_reset(
        counters: &DashMap<String, Arc<AtomicU64>>,
        counter_name: &str,
        new_value: u64,
        db_queue: &Option<Arc<SegQueue<DatabaseOperation>>>,
    ) -> io::Result<()> {
        trace_log!("处理重置计数器: {} = {}", counter_name, new_value);

        // 更新内存中的原子计数器（纯内存操作）
        let counter = counters
            .entry(counter_name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();

        counter.store(new_value, Ordering::SeqCst);

        // 立即向DatabaseWorker发送持久化指令
        if let Some(db_queue) = db_queue {
            let persist_op = DatabaseOperation::PersistCounter {
                counter_name: counter_name.to_string(),
                value: new_value,
                response_tx: std::sync::mpsc::channel().0, // 不需要响应，直接丢弃
            };
            db_queue.push(persist_op);
            trace_log!("已发送持久化指令: {} = {}", counter_name, new_value);
        }

        trace_log!("重置计数器完成: {} = {}", counter_name, new_value);
        Ok(())
    }

    /// 提交原子递增操作
    pub(crate) fn increment(&self, counter_name: String, delta: u64) -> io::Result<u64> {
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
    pub(crate) fn get(&self, counter_name: String) -> io::Result<Option<u64>> {
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

    /// 提交原子递减操作
    pub(crate) fn decrement(&self, counter_name: String, delta: u64) -> io::Result<u64> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Decrement {
            counter_name,
            delta,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交原子乘法操作
    pub(crate) fn multiply(&self, counter_name: String, factor: u64) -> io::Result<u64> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Multiply {
            counter_name,
            factor,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交原子除法操作
    pub(crate) fn divide(&self, counter_name: String, divisor: u64) -> io::Result<u64> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Divide {
            counter_name,
            divisor,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交原子百分比操作
    pub(crate) fn percentage(&self, counter_name: String, percentage: u64) -> io::Result<u64> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::Percentage {
            counter_name,
            percentage,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交原子比较和交换操作
    pub(crate) fn compare_and_swap(&self, counter_name: String, expected: u64, new_value: u64) -> io::Result<bool> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = AtomicOperation::CompareAndSwap {
            counter_name,
            expected,
            new_value,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Worker连接断开"))
        })
    }

    /// 提交重置计数器操作
    pub(crate) fn reset(&self, counter_name: String, new_value: u64) -> io::Result<()> {
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
    pub(crate) fn load_counter(&self, counter_name: String, value: u64) {
        trace_log!("加载计数器: {} = {}", counter_name, value);
        let counter = Arc::new(AtomicU64::new(value));
        self.counters.insert(counter_name, counter);
    }

    /// 获取所有计数器名称（供调试使用）
    pub(crate) fn get_counter_names(&self) -> Vec<String> {
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