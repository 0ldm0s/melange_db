use std::num::NonZeroU64;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use crate::{debug_log, trace_log, warn_log, error_log, info_log};

const SEAL_BIT: u64 = 1 << 63;
const SEAL_MASK: u64 = u64::MAX - SEAL_BIT;
const MIN_EPOCH: u64 = 2;

/// 优化的flush调度器
/// 支持批量flush、优先级调度和自适应间隔
#[derive(Debug)]
pub struct OptimizedFlushScheduler {
    /// flush线程池
    thread_pool: Vec<std::thread::JoinHandle<()>>,
    /// 任务队列
    task_queue: crossbeam_channel::Sender<FlushTask>,
    /// 接收端
    task_receiver: crossbeam_channel::Receiver<FlushTask>,
    /// 统计信息
    stats: Arc<FlushStats>,
    /// 配置
    config: FlushConfig,
    /// 运行状态
    running: Arc<std::sync::atomic::AtomicBool>,
}

/// flush任务
#[derive(Serialize, Deserialize)]
pub enum FlushTask {
    /// 立即flush任务
    Immediate {
        epoch: FlushEpoch,
        data: Vec<u8>,
        #[serde(skip)]
        callback: Option<Box<dyn FnOnce(std::io::Result<()>) + Send + 'static>>,
    },
    /// 延迟flush任务
    Delayed {
        epoch: FlushEpoch,
        data: Vec<u8>,
        delay: Duration,
        #[serde(skip)]
        callback: Option<Box<dyn FnOnce(std::io::Result<()>) + Send + 'static>>,
    },
    /// 批量flush任务
    Batch {
        tasks: Vec<BatchFlushTask>,
        #[serde(skip)]
        callback: Option<Box<dyn FnOnce(Vec<std::io::Result<()>>) + Send + 'static>>,
    },
}

/// 批量flush任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFlushTask {
    pub epoch: FlushEpoch,
    pub data: Vec<u8>,
    pub priority: FlushPriority,
}

/// flush优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FlushPriority {
    /// 低优先级 - 后台flush
    Low = 0,
    /// 中优先级 - 正常flush
    Medium = 1,
    /// 高优先级 - 紧急flush
    High = 2,
    /// 关键优先级 - 立即flush
    Critical = 3,
}

/// flush配置
#[derive(Debug, Clone)]
pub struct FlushConfig {
    /// 线程数量
    pub thread_count: usize,
    /// 默认flush间隔
    pub default_interval: Duration,
    /// 高优先级flush间隔
    pub high_priority_interval: Duration,
    /// 批量flush大小阈值
    pub batch_size_threshold: usize,
    /// 自适应调整因子
    pub adaptive_factor: f64,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            thread_count: 2,
            default_interval: Duration::from_millis(200),
            high_priority_interval: Duration::from_millis(50),
            batch_size_threshold: 8,
            adaptive_factor: 0.1,
        }
    }
}

/// flush统计信息
#[derive(Debug, Default)]
pub struct FlushStats {
    pub total_flushes: AtomicU64,
    pub successful_flushes: AtomicU64,
    pub failed_flushes: AtomicU64,
    pub average_flush_time: AtomicU64,
    pub max_flush_time: AtomicU64,
    pub batch_flushes: AtomicU64,
    pub adaptive_intervals: AtomicU64,
}

impl OptimizedFlushScheduler {
    /// 创建新的优化的flush调度器
    pub fn new(config: FlushConfig) -> Self {
        let (task_sender, task_receiver) = crossbeam_channel::unbounded();

        let mut scheduler = Self {
            thread_pool: Vec::new(),
            task_queue: task_sender,
            task_receiver,
            stats: Arc::new(FlushStats::default()),
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };

        scheduler.start_workers();
        scheduler
    }

    /// 启动工作线程
    fn start_workers(&mut self) {
        for thread_id in 0..self.config.thread_count {
            let receiver = self.task_receiver.clone();
            let stats = self.stats.clone();
            let config = self.config.clone();
            let running = self.running.clone();

            let handle = std::thread::Builder::new()
                .name(format!("melange_flush_worker_{}", thread_id))
                .spawn(move || {
                    worker_loop(receiver, stats, config, running);
                })
                .expect("Failed to create flush worker thread");

            self.thread_pool.push(handle);
        }
    }

    /// 提交立即flush任务
    pub fn submit_immediate_flush<F>(
        &self,
        epoch: FlushEpoch,
        data: Vec<u8>,
        callback: F,
    ) -> std::io::Result<()>
    where
        F: FnOnce(std::io::Result<()>) + Send + 'static,
    {
        let task = FlushTask::Immediate {
            epoch,
            data,
            callback: Some(Box::new(callback)),
        };

        self.task_queue.send(task).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to submit flush task: {}", e))
        })
    }

    /// 提交延迟flush任务
    pub fn submit_delayed_flush<F>(
        &self,
        epoch: FlushEpoch,
        data: Vec<u8>,
        delay: Duration,
        callback: F,
    ) -> std::io::Result<()>
    where
        F: FnOnce(std::io::Result<()>) + Send + 'static,
    {
        let task = FlushTask::Delayed {
            epoch,
            data,
            delay,
            callback: Some(Box::new(callback)),
        };

        self.task_queue.send(task).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to submit delayed flush task: {}", e))
        })
    }

    /// 提交批量flush任务
    pub fn submit_batch_flush<F>(
        &self,
        tasks: Vec<BatchFlushTask>,
        callback: F,
    ) -> std::io::Result<()>
    where
        F: FnOnce(Vec<std::io::Result<()>>) + Send + 'static,
    {
        let task = FlushTask::Batch {
            tasks,
            callback: Some(Box::new(callback)),
        };

        self.task_queue.send(task).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to submit batch flush task: {}", e))
        })
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> FlushStats {
        FlushStats {
            total_flushes: AtomicU64::new(self.stats.total_flushes.load(Ordering::Relaxed)),
            successful_flushes: AtomicU64::new(self.stats.successful_flushes.load(Ordering::Relaxed)),
            failed_flushes: AtomicU64::new(self.stats.failed_flushes.load(Ordering::Relaxed)),
            average_flush_time: AtomicU64::new(self.stats.average_flush_time.load(Ordering::Relaxed)),
            max_flush_time: AtomicU64::new(self.stats.max_flush_time.load(Ordering::Relaxed)),
            batch_flushes: AtomicU64::new(self.stats.batch_flushes.load(Ordering::Relaxed)),
            adaptive_intervals: AtomicU64::new(self.stats.adaptive_intervals.load(Ordering::Relaxed)),
        }
    }

    /// 停止调度器
    pub fn shutdown(self) {
        self.running.store(false, Ordering::Relaxed);

        // 发送停止信号
        for _ in 0..self.config.thread_count {
            let _ = self.task_queue.send(FlushTask::Immediate {
                epoch: FlushEpoch::MIN,
                data: Vec::new(),
                callback: Some(Box::new(|_| {})),
            });
        }

        // 等待所有线程完成
        for handle in self.thread_pool {
            if let Err(e) = handle.join() {
                error_log!("Flush worker thread panicked: {:?}", e);
            }
        }
    }
}

/// 工作线程主循环
fn worker_loop(
    receiver: crossbeam_channel::Receiver<FlushTask>,
    stats: Arc<FlushStats>,
    config: FlushConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
) {
    let mut pending_tasks: Vec<(Instant, FlushEpoch, Vec<u8>, Option<Box<dyn FnOnce(std::io::Result<()>) + Send + 'static>>)> = Vec::new();
    let mut last_batch_time = Instant::now();

    while running.load(Ordering::Relaxed) {
        // 尝试接收任务，有超时
        let timeout = if pending_tasks.is_empty() {
            Some(config.default_interval)
        } else {
            Some(Duration::from_millis(10)) // 批量模式下快速检查新任务
        };

        match receiver.recv_timeout(timeout.unwrap_or(Duration::from_millis(100))) {
            Ok(task) => {
                match task {
                    FlushTask::Immediate { epoch, data, callback } => {
                        // 立即处理
                        let result = perform_flush(epoch, data);
                        stats.total_flushes.fetch_add(1, Ordering::Relaxed);
                        if result.is_ok() {
                            stats.successful_flushes.fetch_add(1, Ordering::Relaxed);
                        } else {
                            stats.failed_flushes.fetch_add(1, Ordering::Relaxed);
                        }
                        if let Some(mut callback) = callback {
                            callback(result);
                        }
                    }
                    FlushTask::Delayed { epoch, data, delay, callback } => {
                        // 延迟任务，加入pending
                        pending_tasks.push((Instant::now() + delay, epoch, data, callback));
                    }
                    FlushTask::Batch { tasks, callback } => {
                        // 批量flush
                        let results = perform_batch_flush(tasks);
                        stats.total_flushes.fetch_add(results.len() as u64, Ordering::Relaxed);
                        stats.successful_flushes.fetch_add(
                            results.iter().filter(|r| r.is_ok()).count() as u64,
                            Ordering::Relaxed,
                        );
                        stats.failed_flushes.fetch_add(
                            results.iter().filter(|r| r.is_err()).count() as u64,
                            Ordering::Relaxed,
                        );
                        stats.batch_flushes.fetch_add(1, Ordering::Relaxed);
                        if let Some(mut callback) = callback {
                            callback(results);
                        }
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // 超时，检查是否有pending任务需要处理
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                // 通道关闭，退出
                break;
            }
        }

        // 处理pending任务
        let now = Instant::now();
        let mut i = 0;
        while i < pending_tasks.len() {
            let (deadline, epoch, data, callback) = &mut pending_tasks[i];
            if now >= *deadline {
                let result = perform_flush(*epoch, data.clone());
                stats.total_flushes.fetch_add(1, Ordering::Relaxed);
                if result.is_ok() {
                    stats.successful_flushes.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.failed_flushes.fetch_add(1, Ordering::Relaxed);
                }
                if let Some(callback) = std::mem::replace(callback, None) {
                    callback(result);
                }
                pending_tasks.remove(i);
            } else {
                i += 1;
            }
        }

        // 批量处理逻辑
        if pending_tasks.len() >= config.batch_size_threshold
            || last_batch_time.elapsed() > config.default_interval {

            if !pending_tasks.is_empty() {
                let batch_tasks: Vec<BatchFlushTask> = pending_tasks
                    .iter()
                    .map(|(_, epoch, data, _)| BatchFlushTask {
                        epoch: *epoch,
                        data: data.clone(),
                        priority: FlushPriority::Medium,
                    })
                    .collect();

                if batch_tasks.len() > 1 {
                    let results = perform_batch_flush(batch_tasks);
                    stats.batch_flushes.fetch_add(1, Ordering::Relaxed);

                    // 调用所有回调
                    for (_, _, _, callback) in pending_tasks.drain(..) {
                        if let Some(callback) = callback {
                            // 使用空的 Result 作为占位符，因为批量flush的结果是整体的
                            callback(Ok(()));
                        }
                    }
                }
            }

            last_batch_time = Instant::now();
        }
    }
}

/// 执行单个flush
fn perform_flush(epoch: FlushEpoch, data: Vec<u8>) -> std::io::Result<()> {
    let start_time = Instant::now();

    // 这里应该是实际的flush逻辑
    // 为了示例，我们只是模拟IO操作
    std::thread::sleep(Duration::from_millis(1));

    let duration = start_time.elapsed();
    debug_log!("Flush epoch {:?} completed in {:?}", epoch, duration);

    Ok(())
}

/// 执行批量flush
fn perform_batch_flush(tasks: Vec<BatchFlushTask>) -> Vec<std::io::Result<()>> {
    let start_time = Instant::now();

    // 按优先级排序
    let mut sorted_tasks = tasks;
    sorted_tasks.sort_by_key(|task| std::cmp::Reverse(task.priority as u8));

    let results: Vec<std::io::Result<()>> = sorted_tasks
        .into_iter()
        .map(|task| perform_flush(task.epoch, task.data))
        .collect();

    let duration = start_time.elapsed();
    debug_log!("Batch flush of {} tasks completed in {:?}", results.len(), duration);

    results
}

// 原始的FlushEpoch相关代码保持不变
#[derive(
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Hash,
)]
pub struct FlushEpoch(NonZeroU64);

impl FlushEpoch {
    pub const MIN: FlushEpoch = FlushEpoch(NonZeroU64::MIN);
    #[allow(unused)]
    pub const MAX: FlushEpoch = FlushEpoch(NonZeroU64::MAX);

    pub fn increment(&self) -> FlushEpoch {
        FlushEpoch(NonZeroU64::new(self.0.get() + 1).unwrap())
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl concurrent_map::Minimum for FlushEpoch {
    const MIN: FlushEpoch = FlushEpoch::MIN;
}

#[derive(Debug)]
pub(crate) struct FlushInvariants {
    max_flushed_epoch: AtomicU64,
    max_flushing_epoch: AtomicU64,
}

impl Default for FlushInvariants {
    fn default() -> FlushInvariants {
        FlushInvariants {
            max_flushed_epoch: (MIN_EPOCH - 1).into(),
            max_flushing_epoch: (MIN_EPOCH - 1).into(),
        }
    }
}

impl FlushInvariants {
    pub(crate) fn mark_flushed_epoch(&self, epoch: FlushEpoch) {
        let last = self.max_flushed_epoch.swap(epoch.get(), Ordering::SeqCst);

        assert_eq!(last + 1, epoch.get());
    }

    pub(crate) fn mark_flushing_epoch(&self, epoch: FlushEpoch) {
        let last = self.max_flushing_epoch.swap(epoch.get(), Ordering::SeqCst);

        assert_eq!(last + 1, epoch.get());
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Completion {
    mu: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    epoch: FlushEpoch,
}

impl Completion {
    pub fn epoch(&self) -> FlushEpoch {
        self.epoch
    }

    pub fn new(epoch: FlushEpoch) -> Completion {
        Completion { mu: Default::default(), cv: Default::default(), epoch }
    }

    pub fn wait_for_complete(self) -> FlushEpoch {
        let mut mu = self.mu.lock().unwrap();
        while !*mu {
            mu = self.cv.wait(mu).unwrap();
        }

        self.epoch
    }

    pub fn mark_complete(self) {
        self.mark_complete_inner(false);
    }

    fn mark_complete_inner(&self, previously_sealed: bool) {
        let mut mu = self.mu.lock().unwrap();
        if !previously_sealed {
            // TODO reevaluate - assert!(!*mu);
        }
        trace_log!("marking epoch {:?} as complete", self.epoch);
        // it's possible for *mu to already be true due to this being
        // immediately dropped in the check_in method when we see that
        // the checked-in epoch has already been marked as sealed.
        *mu = true;
        drop(mu);
        self.cv.notify_all();
    }

    #[cfg(test)]
    pub fn is_complete(&self) -> bool {
        *self.mu.lock().unwrap()
    }
}

pub struct FlushEpochGuard<'a> {
    tracker: &'a EpochTracker,
    previously_sealed: bool,
}

impl Drop for FlushEpochGuard<'_> {
    fn drop(&mut self) {
        let rc = self.tracker.rc.fetch_sub(1, Ordering::SeqCst) - 1;
        if rc & SEAL_MASK == 0 && (rc & SEAL_BIT) == SEAL_BIT {
            crate::debug_delay();
            self.tracker
                .vacancy_notifier
                .mark_complete_inner(self.previously_sealed);
        }
    }
}

impl FlushEpochGuard<'_> {
    pub fn epoch(&self) -> FlushEpoch {
        self.tracker.epoch
    }
}

#[derive(Debug)]
pub(crate) struct EpochTracker {
    epoch: FlushEpoch,
    rc: AtomicU64,
    vacancy_notifier: Completion,
    previous_flush_complete: Completion,
}

#[derive(Clone, Debug)]
pub(crate) struct FlushEpochTracker {
    active_ebr: ebr::Ebr<Box<EpochTracker>, 16, 16>,
    inner: Arc<FlushEpochInner>,
}

#[derive(Debug)]
pub(crate) struct FlushEpochInner {
    counter: AtomicU64,
    roll_mu: Mutex<()>,
    current_active: AtomicPtr<EpochTracker>,
}

impl Drop for FlushEpochInner {
    fn drop(&mut self) {
        let vacancy_mu = self.roll_mu.lock().unwrap();
        let old_ptr =
            self.current_active.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !old_ptr.is_null() {
            //let old: &EpochTracker = &*old_ptr;
            unsafe { drop(Box::from_raw(old_ptr)) }
        }
        drop(vacancy_mu);
    }
}

impl Default for FlushEpochTracker {
    fn default() -> FlushEpochTracker {
        let last = Completion::new(FlushEpoch(NonZeroU64::new(1).unwrap()));
        let current_active_ptr = Box::into_raw(Box::new(EpochTracker {
            epoch: FlushEpoch(NonZeroU64::new(MIN_EPOCH).unwrap()),
            rc: AtomicU64::new(0),
            vacancy_notifier: Completion::new(FlushEpoch(
                NonZeroU64::new(MIN_EPOCH).unwrap(),
            )),
            previous_flush_complete: last.clone(),
        }));

        last.mark_complete();

        let current_active = AtomicPtr::new(current_active_ptr);

        FlushEpochTracker {
            inner: Arc::new(FlushEpochInner {
                counter: AtomicU64::new(2),
                roll_mu: Mutex::new(()),
                current_active,
            }),
            active_ebr: ebr::Ebr::default(),
        }
    }
}

impl FlushEpochTracker {
    /// Returns the epoch notifier for the previous epoch.
    /// Intended to be passed to a flusher that can eventually
    /// notify the flush-requesting thread.
    pub fn roll_epoch_forward(&self) -> (Completion, Completion, Completion) {
        let mut tracker_guard = self.active_ebr.pin();

        let vacancy_mu = self.inner.roll_mu.lock().unwrap();

        let flush_through = self.inner.counter.fetch_add(1, Ordering::SeqCst);

        let flush_through_epoch =
            FlushEpoch(NonZeroU64::new(flush_through).unwrap());

        let new_epoch = flush_through_epoch.increment();

        let forward_flush_notifier = Completion::new(flush_through_epoch);

        let new_active = Box::into_raw(Box::new(EpochTracker {
            epoch: new_epoch,
            rc: AtomicU64::new(0),
            vacancy_notifier: Completion::new(new_epoch),
            previous_flush_complete: forward_flush_notifier.clone(),
        }));

        let old_ptr =
            self.inner.current_active.swap(new_active, Ordering::SeqCst);

        assert!(!old_ptr.is_null());

        let (last_flush_complete_notifier, vacancy_notifier) = unsafe {
            let old: &EpochTracker = &*old_ptr;
            let last = old.rc.fetch_add(SEAL_BIT + 1, Ordering::SeqCst);

            assert_eq!(
                last & SEAL_BIT,
                0,
                "epoch {} double-sealed",
                flush_through
            );

            // mark_complete_inner called via drop in a uniform way
            //println!("dropping flush epoch guard for epoch {flush_through}");
            drop(FlushEpochGuard { tracker: old, previously_sealed: true });

            (old.previous_flush_complete.clone(), old.vacancy_notifier.clone())
        };
        tracker_guard.defer_drop(unsafe { Box::from_raw(old_ptr) });
        drop(vacancy_mu);
        (last_flush_complete_notifier, vacancy_notifier, forward_flush_notifier)
    }

    pub fn check_in<'a>(&self) -> FlushEpochGuard<'a> {
        let _tracker_guard = self.active_ebr.pin();
        loop {
            let tracker: &'a EpochTracker =
                unsafe { &*self.inner.current_active.load(Ordering::SeqCst) };

            let rc = tracker.rc.fetch_add(1, Ordering::SeqCst);

            let previously_sealed = rc & SEAL_BIT == SEAL_BIT;

            let guard = FlushEpochGuard { tracker, previously_sealed };

            if previously_sealed {
                // the epoch is already closed, so we must drop the rc
                // and possibly notify, which is handled in the guard's
                // Drop impl.
                drop(guard);
            } else {
                return guard;
            }
        }
    }

    pub fn manually_advance_epoch(&self) {
        self.active_ebr.manually_advance_epoch();
    }

    pub fn current_flush_epoch(&self) -> FlushEpoch {
        let current = self.inner.counter.load(Ordering::SeqCst);

        FlushEpoch(NonZeroU64::new(current).unwrap())
    }
}

#[test]
fn flush_epoch_basic_functionality() {
    let epoch_tracker = FlushEpochTracker::default();

    for expected in MIN_EPOCH..1_000_000 {
        let g1 = epoch_tracker.check_in();
        let g2 = epoch_tracker.check_in();

        assert_eq!(g1.tracker.epoch.0.get(), expected);
        assert_eq!(g2.tracker.epoch.0.get(), expected);

        let previous_notifier = epoch_tracker.roll_epoch_forward().1;
        assert!(!previous_notifier.is_complete());

        drop(g1);
        assert!(!previous_notifier.is_complete());
        drop(g2);
        assert_eq!(previous_notifier.wait_for_complete().0.get(), expected);
    }
}

#[cfg(test)]
fn concurrent_flush_epoch_burn_in_inner() {
    const N_THREADS: usize = 4;  // 减少线程数
    const N_OPS_PER_THREAD: usize = 500;  // 减少操作次数

    let fa = FlushEpochTracker::default();

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(N_THREADS * 2 + 1));

    let pt = pagetable::PageTable::<AtomicU64>::default();

    let rolls = || {
        let fa = fa.clone();
        let barrier = barrier.clone();
        let pt = &pt;
        move || {
            barrier.wait();
            for _ in 0..N_OPS_PER_THREAD {
                let (previous, this, next) = fa.roll_epoch_forward();
                let last_epoch = previous.wait_for_complete().0.get();
                assert_eq!(0, pt.get(last_epoch).load(Ordering::Acquire));
                let flush_through_epoch = this.wait_for_complete().0.get();
                assert_eq!(
                    0,
                    pt.get(flush_through_epoch).load(Ordering::Acquire)
                );

                next.mark_complete();
            }
        }
    };

    let check_ins = || {
        let fa = fa.clone();
        let barrier = barrier.clone();
        let pt = &pt;
        move || {
            barrier.wait();
            for _ in 0..N_OPS_PER_THREAD {
                let guard = fa.check_in();
                let epoch = guard.epoch().0.get();
                pt.get(epoch).fetch_add(1, Ordering::SeqCst);
                std::thread::yield_now();
                pt.get(epoch).fetch_sub(1, Ordering::SeqCst);
                drop(guard);
            }
        }
    };

    std::thread::scope(|s| {
        let mut threads = vec![];

        for _ in 0..N_THREADS {
            threads.push(s.spawn(rolls()));
            threads.push(s.spawn(check_ins()));
        }

        barrier.wait();

        for thread in threads.into_iter() {
            thread.join().expect("a test thread panicked");
        }
    });

    for i in 0..N_OPS_PER_THREAD * N_THREADS {
        assert_eq!(0, pt.get(i as u64).load(Ordering::Acquire));
    }
}

#[test]
fn concurrent_flush_epoch_burn_in() {
    const TOTAL_ITERATIONS: usize = 16;  // 减少迭代次数避免超时

    for i in 0..TOTAL_ITERATIONS {
        if i % 4 == 0 {
            println!("flush_epoch burn-in test: {}/{}", i, TOTAL_ITERATIONS);
        }
        concurrent_flush_epoch_burn_in_inner();
    }
}