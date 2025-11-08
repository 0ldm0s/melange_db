//! 数据库操作Worker
//!
//! 专门处理所有数据库操作，避免与原子操作Worker产生EBR冲突

use std::sync::Arc;
use std::thread;
use std::io;

use crossbeam_queue::SegQueue;
use parking_lot::Mutex;

use crate::{debug_log, trace_log, warn_log, error_log, info_log, InlineArray};
use crate::db::Db;

/// 数据库操作类型
#[derive(Debug, Clone)]
pub(crate) enum DatabaseOperation {
    /// 插入数据
    Insert {
        key: Vec<u8>,
        value: Vec<u8>,
        response_tx: std::sync::mpsc::Sender<io::Result<Option<InlineArray>>>,
    },
    /// 获取数据
    Get {
        key: Vec<u8>,
        response_tx: std::sync::mpsc::Sender<io::Result<Option<InlineArray>>>,
    },
    /// 原子计数器持久化
    PersistCounter {
        counter_name: String,
        value: u64,
        response_tx: std::sync::mpsc::Sender<io::Result<()>>,
    },
    /// 预热计数器
    PreloadCounters {
        response_tx: std::sync::mpsc::Sender<io::Result<Vec<(String, u64)>>>,
    },
    /// 扫描前缀
    ScanPrefix {
        prefix: Vec<u8>,
        response_tx: std::sync::mpsc::Sender<io::Result<Vec<(Vec<u8>, Vec<u8>)>>>,
    },
    /// 删除数据
    Remove {
        key: Vec<u8>,
        response_tx: std::sync::mpsc::Sender<io::Result<Option<InlineArray>>>,
    },
    /// 检查键是否存在
    ContainsKey {
        key: Vec<u8>,
        response_tx: std::sync::mpsc::Sender<io::Result<bool>>,
    },
    /// 清空所有数据
    Clear {
        response_tx: std::sync::mpsc::Sender<io::Result<()>>,
    },
    /// 获取键值对总数
    Len {
        response_tx: std::sync::mpsc::Sender<io::Result<usize>>,
    },
    /// 检查是否为空
    IsEmpty {
        response_tx: std::sync::mpsc::Sender<io::Result<bool>>,
    },
    /// 获取第一个键值对
    First {
        response_tx: std::sync::mpsc::Sender<io::Result<Option<(InlineArray, InlineArray)>>>,
    },
    /// 获取最后一个键值对
    Last {
        response_tx: std::sync::mpsc::Sender<io::Result<Option<(InlineArray, InlineArray)>>>,
    },
}

/// 数据库操作Worker
///
/// 专门处理所有数据库操作，与原子操作完全解耦
pub(crate) struct DatabaseWorker {
    /// 操作队列 (无锁并发队列)
    operation_queue: Arc<SegQueue<DatabaseOperation>>,

    /// Worker句柄
    worker_handle: Option<thread::JoinHandle<()>>,

    /// 关闭信号
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl DatabaseWorker {
    /// 创建新的数据库操作Worker
    ///
    /// # Arguments
    /// * `db` - 数据库实例引用
    pub(crate) fn new(db: Arc<Db<1024>>) -> Self {
        let operation_queue = Arc::new(SegQueue::new());
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

        let worker_queue = operation_queue.clone();

        let worker_handle = thread::spawn(move || {
            debug_log!("数据库操作Worker线程启动");
            Self::worker_loop(worker_queue, db, shutdown_rx);
            debug_log!("数据库操作Worker线程退出");
        });

        Self {
            operation_queue,
            worker_handle: Some(worker_handle),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Worker主循环
    fn worker_loop(
        operation_queue: Arc<SegQueue<DatabaseOperation>>,
        db: Arc<Db<1024>>,
        shutdown_rx: std::sync::mpsc::Receiver<()>,
    ) {
        loop {
            // 检查关闭信号
            match shutdown_rx.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    debug_log!("收到关闭信号，DatabaseWorker退出");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 继续处理操作
                }
            }

            // 处理操作队列
            if let Some(operation) = operation_queue.pop() {
                Self::handle_operation(&db, operation);
            } else {
                // 队列为空，短暂休眠避免CPU占用过高
                thread::yield_now();
            }
        }
    }

    /// 处理单个数据库操作
    fn handle_operation(db: &Db<1024>, operation: DatabaseOperation) {
        match operation {
            DatabaseOperation::Insert { key, value, response_tx } => {
                let result = db.insert(&key, &*value);
                let _ = response_tx.send(result);
            }
            DatabaseOperation::Get { key, response_tx } => {
                let result = db.get(&key);
                let _ = response_tx.send(result);
            }
            DatabaseOperation::PersistCounter { counter_name, value, response_tx } => {
                trace_log!("持久化计数器: {} = {}", counter_name, value);
                let key = format!("__atomic_counter__:{}", counter_name);
                let result = db.insert(key.as_bytes(), &value.to_le_bytes()).map(|_| ());
                let _ = response_tx.send(result);
            }
            DatabaseOperation::PreloadCounters { response_tx } => {
                debug_log!("开始预热计数器...");
                let mut counters = Vec::new();

                let prefix = b"__atomic_counter__:";
                for item_res in db.scan_prefix(prefix) {
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

                debug_log!("预热完成，加载了 {} 个计数器", counters.len());
                let _ = response_tx.send(Ok(counters));
            }
            DatabaseOperation::ScanPrefix { prefix, response_tx } => {
                let result = db.scan_prefix(&prefix)
                    .collect::<io::Result<Vec<_>>>()
                    .map(|items| {
                        items.into_iter()
                            .map(|(key, value)| (key.to_vec(), value.to_vec()))
                            .collect()
                    });
                let _ = response_tx.send(result);
            }
            DatabaseOperation::Remove { key, response_tx } => {
                let result = db.remove(&key);
                let _ = response_tx.send(result);
            }
            DatabaseOperation::ContainsKey { key, response_tx } => {
                let result = db.contains_key(&key);
                let _ = response_tx.send(result);
            }
            DatabaseOperation::Clear { response_tx } => {
                let result = db.clear();
                let _ = response_tx.send(result);
            }
            DatabaseOperation::Len { response_tx } => {
                let result = db.len();
                let _ = response_tx.send(result);
            }
            DatabaseOperation::IsEmpty { response_tx } => {
                let result = db.is_empty();
                let _ = response_tx.send(result);
            }
            DatabaseOperation::First { response_tx } => {
                let result = db.first();
                let _ = response_tx.send(result);
            }
            DatabaseOperation::Last { response_tx } => {
                let result = db.last();
                let _ = response_tx.send(result);
            }
        }
    }

    /// 提交插入操作
    pub(crate) fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> io::Result<Option<InlineArray>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Insert {
            key,
            value,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交获取操作
    pub(crate) fn get(&self, key: Vec<u8>) -> io::Result<Option<InlineArray>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Get {
            key,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交原子计数器持久化操作
    pub(crate) fn persist_counter(&self, counter_name: String, value: u64) -> io::Result<()> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::PersistCounter {
            counter_name,
            value,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交预热计数器操作
    pub(crate) fn preload_counters(&self) -> io::Result<Vec<(String, u64)>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::PreloadCounters {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交扫描前缀操作
    pub(crate) fn scan_prefix(&self, prefix: Vec<u8>) -> io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::ScanPrefix {
            prefix,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交删除操作
    pub(crate) fn remove(&self, key: Vec<u8>) -> io::Result<Option<InlineArray>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Remove {
            key,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交检查键是否存在操作
    pub(crate) fn contains_key(&self, key: Vec<u8>) -> io::Result<bool> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::ContainsKey {
            key,
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交清空操作
    pub(crate) fn clear(&self) -> io::Result<()> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Clear {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交获取键值对总数操作
    pub(crate) fn len(&self) -> io::Result<usize> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Len {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交检查是否为空操作
    pub(crate) fn is_empty(&self) -> io::Result<bool> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::IsEmpty {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交获取第一个键值对操作
    pub(crate) fn first(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::First {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 提交获取最后一个键值对操作
    pub(crate) fn last(&self) -> io::Result<Option<(InlineArray, InlineArray)>> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let operation = DatabaseOperation::Last {
            response_tx,
        };

        self.operation_queue.push(operation);

        response_rx.recv().unwrap_or_else(|_| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "DatabaseWorker连接断开"))
        })
    }

    /// 获取操作队列引用（供其他Worker使用）
    pub(crate) fn operation_queue(&self) -> &Arc<SegQueue<DatabaseOperation>> {
        &self.operation_queue
    }
}

impl Drop for DatabaseWorker {
    fn drop(&mut self) {
        debug_log!("开始关闭数据库操作Worker");

        // 发送关闭信号
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // 等待Worker线程退出
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        debug_log!("数据库操作Worker已关闭");
    }
}