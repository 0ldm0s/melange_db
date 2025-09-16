use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::{debug_log};

/// 智能flush策略配置
#[derive(Debug, Clone)]
pub struct SmartFlushConfig {
    /// 基础flush间隔（毫秒）
    pub base_interval_ms: usize,
    /// 最小flush间隔（毫秒）
    pub min_interval_ms: usize,
    /// 最大flush间隔（毫秒）
    pub max_interval_ms: usize,
    /// 写入速率阈值（ops/sec），超过此值则更频繁flush
    pub write_rate_threshold: u64,
    /// 累积写入量阈值（bytes），超过此值则立即flush
    pub accumulated_bytes_threshold: usize,
    /// 是否启用自适应flush
    pub enabled: bool,
}

impl Default for SmartFlushConfig {
    fn default() -> Self {
        Self {
            base_interval_ms: 200,  // 默认200ms
            min_interval_ms: 50,   // 最小50ms
            max_interval_ms: 2000, // 最大2秒
            write_rate_threshold: 10000, // 10K ops/sec
            accumulated_bytes_threshold: 4 * 1024 * 1024, // 4MB
            enabled: true,
        }
    }
}

/// 写入负载统计（内部实现细节）
#[doc(hidden)]
#[derive(Debug)]
pub struct WriteLoadStats {
    /// 写入操作计数
    write_count: AtomicU64,
    /// 写入字节数
    write_bytes: AtomicU64,
    /// 上次统计时间
    last_stats_time: RwLock<Instant>,
    /// 当前写入速率（ops/sec）
    current_write_rate: AtomicU64,
    /// 当前写入字节速率（bytes/sec）
    current_byte_rate: AtomicU64,
    /// 累积未flush的字节数
    accumulated_bytes: AtomicUsize,
}

impl WriteLoadStats {
    pub fn new() -> Self {
        Self {
            write_count: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            last_stats_time: RwLock::new(Instant::now()),
            current_write_rate: AtomicU64::new(0),
            current_byte_rate: AtomicU64::new(0),
            accumulated_bytes: AtomicUsize::new(0),
        }
    }

    /// 记录写入操作
    pub fn record_write(&self, bytes_written: usize) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.write_bytes.fetch_add(bytes_written as u64, Ordering::Relaxed);
        self.accumulated_bytes.fetch_add(bytes_written, Ordering::Relaxed);
    }

    /// 更新写入速率统计
    pub fn update_rates(&self) {
        let now = Instant::now();
        let mut last_time = self.last_stats_time.write();

        let elapsed = now.duration_since(*last_time);
        if elapsed.as_secs() > 0 {
            let write_count = self.write_count.swap(0, Ordering::Relaxed);
            let write_bytes = self.write_bytes.swap(0, Ordering::Relaxed);

            let write_rate = (write_count as f64 / elapsed.as_secs_f64()) as u64;
            let byte_rate = (write_bytes as f64 / elapsed.as_secs_f64()) as u64;

            self.current_write_rate.store(write_rate, Ordering::Relaxed);
            self.current_byte_rate.store(byte_rate, Ordering::Relaxed);
        }

        *last_time = now;
    }

    /// 获取当前写入速率
    pub fn get_write_rate(&self) -> u64 {
        self.current_write_rate.load(Ordering::Relaxed)
    }

    /// 获取当前字节速率
    pub fn get_byte_rate(&self) -> u64 {
        self.current_byte_rate.load(Ordering::Relaxed)
    }

    /// 获取累积字节数
    pub fn get_accumulated_bytes(&self) -> usize {
        self.accumulated_bytes.load(Ordering::Relaxed)
    }

    /// 重置累积字节数
    pub fn reset_accumulated_bytes(&self) {
        self.accumulated_bytes.store(0, Ordering::Relaxed);
    }
}

/// 智能flush调度器（内部实现细节）
#[doc(hidden)]
pub struct SmartFlushScheduler {
    config: SmartFlushConfig,
    stats: Arc<WriteLoadStats>,
    last_flush_time: RwLock<Instant>,
}

impl SmartFlushScheduler {
    pub fn new(config: SmartFlushConfig) -> Self {
        Self {
            config,
            stats: Arc::new(WriteLoadStats::new()),
            last_flush_time: RwLock::new(Instant::now()),
        }
    }

    /// 获取统计信息引用
    pub fn get_stats(&self) -> Arc<WriteLoadStats> {
        self.stats.clone()
    }

    /// 计算下次flush的延迟时间
    pub fn calculate_next_flush_delay(&self) -> Duration {
        if !self.config.enabled {
            return Duration::from_millis(self.config.base_interval_ms as u64);
        }

        // 更新写入速率统计
        self.stats.update_rates();

        let write_rate = self.stats.get_write_rate();
        let accumulated_bytes = self.stats.get_accumulated_bytes();
        let last_flush = *self.last_flush_time.read();
        let time_since_last_flush = Instant::now().duration_since(last_flush);

        // 策略1：检查累积字节数是否超过阈值
        if accumulated_bytes >= self.config.accumulated_bytes_threshold {
            debug_log!("智能flush: 累积字节{}超过阈值{}, 立即flush",
                      accumulated_bytes, self.config.accumulated_bytes_threshold);
            return Duration::from_millis(0);
        }

        // 策略2：基于写入速率调整flush间隔
        let mut interval_ms = self.config.base_interval_ms;

        if write_rate > self.config.write_rate_threshold {
            // 高写入负载：更频繁flush
            let load_factor = (write_rate as f64 / self.config.write_rate_threshold as f64).min(5.0);
            interval_ms = (self.config.base_interval_ms as f64 / load_factor) as usize;
            interval_ms = interval_ms.max(self.config.min_interval_ms);

            debug_log!("智能flush: 高写入负载{} ops/sec, 调整间隔为{}ms",
                      write_rate, interval_ms);
        } else {
            // 低写入负载：可以延长flush间隔
            let load_factor = (write_rate as f64 / self.config.write_rate_threshold as f64).max(0.1);
            interval_ms = (self.config.base_interval_ms as f64 * (2.0 - load_factor)) as usize;
            interval_ms = interval_ms.min(self.config.max_interval_ms);

            debug_log!("智能flush: 低写入负载{} ops/sec, 调整间隔为{}ms",
                      write_rate, interval_ms);
        }

        // 计算还需要等待的时间
        let remaining_interval = Duration::from_millis(interval_ms as u64);

        if time_since_last_flush >= remaining_interval {
            Duration::from_millis(0)  // 立即flush
        } else {
            remaining_interval - time_since_last_flush
        }
    }

    /// 通知flush完成
    pub fn notify_flush_completed(&self) {
        *self.last_flush_time.write() = Instant::now();
        self.stats.reset_accumulated_bytes();
    }

    /// 更新配置
    pub fn update_config(&mut self, config: SmartFlushConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_load_stats() {
        let stats = WriteLoadStats::new();

        // 记录一些写入
        stats.record_write(100);
        stats.record_write(200);
        stats.record_write(150);

        // 更新速率
        stats.update_rates();

        // 检查累积字节数
        assert_eq!(stats.get_accumulated_bytes(), 450);

        // 重置累积字节数
        stats.reset_accumulated_bytes();
        assert_eq!(stats.get_accumulated_bytes(), 0);
    }

    #[test]
    fn test_smart_flush_scheduler() {
        let config = SmartFlushConfig {
            base_interval_ms: 100,
            min_interval_ms: 50,
            max_interval_ms: 500,
            write_rate_threshold: 1000,
            accumulated_bytes_threshold: 1000,
            enabled: true,
        };

        let scheduler = SmartFlushScheduler::new(config);
        let stats = scheduler.get_stats();

        // 测试累积字节触发flush
        stats.record_write(1200);
        let delay = scheduler.calculate_next_flush_delay();
        assert_eq!(delay, Duration::from_millis(0));

        // 重置
        scheduler.notify_flush_completed();

        // 测试正常延迟
        let delay = scheduler.calculate_next_flush_delay();
        assert!(delay > Duration::from_millis(0));
    }
}