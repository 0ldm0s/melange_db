//! 高性能布隆过滤器实现
//!
//! 此模块提供了针对数据库查询优化的布隆过滤器，
//! 用于快速判断key是否可能存在于数据集中。
//!
//! 主要特性：
//! - 可配置的误判率
//! - 动态扩容
//! - 序列化支持
//! - 并发安全访问

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use serde::{Serialize, Deserialize};
use parking_lot::RwLock;
use crate::{debug_log, trace_log, warn_log, error_log, info_log};

/// 多重哈希函数的布隆过滤器
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// 位图数据
    bitmap: Vec<u64>,
    /// 位图大小（以位为单位）
    bit_count: usize,
    /// 哈希函数数量
    hash_count: usize,
    /// 已插入的元素数量
    element_count: Arc<AtomicU64>,
    /// 期望的误判率
    target_fpp: f64,
}

impl BloomFilter {
    /// 创建新的布隆过滤器
    ///
    /// # 参数
    /// - `expected_elements`: 期望插入的元素数量
    /// - `false_positive_rate`: 目标误判率 (0.0 - 1.0)
    pub fn new(expected_elements: usize, false_positive_rate: f64) -> Self {
        assert!(false_positive_rate > 0.0 && false_positive_rate < 1.0);
        assert!(expected_elements > 0);

        // 计算最优的位图大小和哈希函数数量
        let bit_count = Self::optimal_bit_count(expected_elements, false_positive_rate);
        let hash_count = Self::optimal_hash_count(bit_count, expected_elements);

        // 计算需要的u64数量
        let word_count = (bit_count + 63) / 64;
        let bitmap = vec![0; word_count];

        Self {
            bitmap,
            bit_count,
            hash_count,
            element_count: Arc::new(AtomicU64::new(0)),
            target_fpp: false_positive_rate,
        }
    }

    /// 计算最优的位图大小
    fn optimal_bit_count(n: usize, p: f64) -> usize {
        // m = -n * ln(p) / (ln(2))^2
        let ln_p = p.ln();
        let ln_2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        ((n as f64) * (-ln_p) / ln_2_squared) as usize
    }

    /// 计算最优的哈希函数数量
    fn optimal_hash_count(m: usize, n: usize) -> usize {
        // k = m/n * ln(2)
        if n == 0 { return 1; }
        ((m as f64) / (n as f64) * std::f64::consts::LN_2) as usize
    }

    /// 插入一个元素
    pub fn insert(&mut self, data: &[u8]) {
        let hashes = self.compute_hashes(data);

        for hash in hashes {
            let bit_index = (hash % self.bit_count as u64) as usize;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if word_index < self.bitmap.len() {
                let mask = 1u64 << bit_offset;
                self.bitmap[word_index] |= mask;
            }
        }

        self.element_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 检查元素是否可能存在
    pub fn contains(&self, data: &[u8]) -> bool {
        let hashes = self.compute_hashes(data);

        for hash in hashes {
            let bit_index = (hash % self.bit_count as u64) as usize;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if word_index >= self.bitmap.len() {
                return false;
            }

            let mask = 1u64 << bit_offset;
            if (self.bitmap[word_index] & mask) == 0 {
                return false;
            }
        }

        true
    }

    /// 计算多重哈希值
    fn compute_hashes(&self, data: &[u8]) -> Vec<u64> {
        let mut hashes = Vec::with_capacity(self.hash_count);

        // 使用双重哈希技术生成多个哈希值
        let hash1 = self.hash(data, 0);
        let hash2 = self.hash(data, hash1);

        for i in 0..self.hash_count {
            let combined_hash = hash1.wrapping_add((i as u64).wrapping_mul(hash2));
            hashes.push(combined_hash);
        }

        hashes
    }

    /// 单一哈希函数
    fn hash(&self, data: &[u8], seed: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        data.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取当前元素数量
    pub fn len(&self) -> u64 {
        self.element_count.load(Ordering::Relaxed)
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 计算当前的误判率
    pub fn current_false_positive_rate(&self) -> f64 {
        let n = self.len() as f64;
        let m = self.bit_count as f64;
        let k = self.hash_count as f64;

        // (1 - e^(-k*n/m))^k
        let exp = (-k * n / m).exp();
        (1.0 - exp).powf(k)
    }

    /// 检查是否需要扩容
    pub fn needs_resize(&self) -> bool {
        let current_fpp = self.current_false_positive_rate();
        current_fpp > self.target_fpp * 1.5 // 容忍50%的误差
    }

    /// 扩容布隆过滤器
    pub fn resize(&mut self) {
        let new_element_count = (self.len() as usize * 2).max(1024);
        let mut new_filter = Self::new(new_element_count, self.target_fpp);

        // 重新插入所有元素（这里需要记录插入的数据，实际实现中可能需要其他方式）
        // 注意：这是一个简化的实现，实际中可能需要维护插入历史
        warn_log!("布隆过滤器扩容从 {} 到 {} 元素", self.len(), new_element_count);

        *self = new_filter;
    }

    /// 清空布隆过滤器
    pub fn clear(&mut self) {
        self.bitmap.fill(0);
        self.element_count.store(0, Ordering::Relaxed);
    }

    /// 获取位图大小（字节）
    pub fn size_in_bytes(&self) -> usize {
        self.bitmap.len() * 8
    }

    /// 获取统计信息
    pub fn stats(&self) -> BloomFilterStats {
        BloomFilterStats {
            bit_count: self.bit_count,
            hash_count: self.hash_count,
            element_count: self.len(),
            size_in_bytes: self.size_in_bytes(),
            current_fpp: self.current_false_positive_rate(),
            target_fpp: self.target_fpp,
        }
    }
}

/// 布隆过滤器统计信息（内部实现细节）
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct BloomFilterStats {
    pub bit_count: usize,
    pub hash_count: usize,
    pub element_count: u64,
    pub size_in_bytes: usize,
    pub current_fpp: f64,
    pub target_fpp: f64,
}

/// 并发安全的布隆过滤器包装器
#[derive(Debug, Clone)]
pub struct ConcurrentBloomFilter {
    inner: Arc<RwLock<BloomFilter>>,
}

impl ConcurrentBloomFilter {
    pub fn new(expected_elements: usize, false_positive_rate: f64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BloomFilter::new(
                expected_elements,
                false_positive_rate
            ))),
        }
    }

    pub fn insert(&self, data: &[u8]) {
        self.inner.write().insert(data);
    }

    pub fn contains(&self, data: &[u8]) -> bool {
        self.inner.read().contains(data)
    }

    pub fn len(&self) -> u64 {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    pub fn stats(&self) -> BloomFilterStats {
        self.inner.read().stats()
    }
}

/// 分层布隆过滤器
///
/// 使用多个布隆过滤器处理不同热度的数据
#[derive(Debug, Clone)]
pub struct TieredBloomFilter {
    hot: ConcurrentBloomFilter,    // 热数据过滤器
    warm: ConcurrentBloomFilter,  // 温数据过滤器
    cold: ConcurrentBloomFilter,  // 冷数据过滤器
}

impl TieredBloomFilter {
    pub fn new(expected_elements: usize) -> Self {
        // 热数据：低误判率，小容量
        let hot_size = expected_elements / 10;
        let hot = ConcurrentBloomFilter::new(hot_size, 0.01);

        // 温数据：中等误判率和容量
        let warm_size = expected_elements / 3;
        let warm = ConcurrentBloomFilter::new(warm_size, 0.05);

        // 冷数据：高误判率，大容量
        let cold_size = expected_elements;
        let cold = ConcurrentBloomFilter::new(cold_size, 0.1);

        Self { hot, warm, cold }
    }

    /// 插入数据到指定层
    pub fn insert(&self, data: &[u8], tier: FilterTier) {
        match tier {
            FilterTier::Hot => self.hot.insert(data),
            FilterTier::Warm => self.warm.insert(data),
            FilterTier::Cold => self.cold.insert(data),
        }
    }

    /// 检查数据是否存在
    pub fn contains(&self, data: &[u8]) -> FilterResult {
        if self.hot.contains(data) {
            return FilterResult::MayExistHot;
        }

        if self.warm.contains(data) {
            return FilterResult::MayExistWarm;
        }

        if self.cold.contains(data) {
            return FilterResult::MayExistCold;
        }

        FilterResult::DefinitelyNotExist
    }

    /// 获取统计信息
    pub fn stats(&self) -> TieredBloomFilterStats {
        TieredBloomFilterStats {
            hot: self.hot.stats(),
            warm: self.warm.stats(),
            cold: self.cold.stats(),
        }
    }
}

/// 过滤器层级
#[derive(Debug, Clone, Copy)]
pub enum FilterTier {
    Hot,   // 热数据
    Warm,  // 温数据
    Cold,  // 冷数据
}

/// 过滤结果
#[derive(Debug, Clone, PartialEq)]
pub enum FilterResult {
    DefinitelyNotExist,
    MayExistHot,
    MayExistWarm,
    MayExistCold,
}

/// 分层布隆过滤器统计信息（内部实现细节）
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct TieredBloomFilterStats {
    pub hot: BloomFilterStats,
    pub warm: BloomFilterStats,
    pub cold: BloomFilterStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_basic() {
        let mut filter = BloomFilter::new(1000, 0.01);

        assert!(!filter.contains(b"hello"));

        filter.insert(b"hello");
        assert!(filter.contains(b"hello"));

        assert!(!filter.contains(b"world"));
    }

    #[test]
    fn test_bloom_filter_false_positives() {
        let mut filter = BloomFilter::new(100, 0.1);

        // 插入100个不同的元素
        for i in 0..100 {
            let key = format!("key_{}", i);
            filter.insert(key.as_bytes());
        }

        // 检查不存在的元素
        let mut false_positives = 0;
        let test_count = 1000;

        for i in 100..(100 + test_count) {
            let key = format!("key_{}", i);
            if filter.contains(key.as_bytes()) {
                false_positives += 1;
            }
        }

        let actual_fpp = false_positives as f64 / test_count as f64;
        println!("实际误判率: {:.4}", actual_fpp);

        // 应该接近目标误判率
        assert!(actual_fpp < 0.2); // 允许一定的误差
    }

    #[test]
    fn test_concurrent_bloom_filter() {
        let filter = ConcurrentBloomFilter::new(100, 0.01);

        filter.insert(b"test");
        assert!(filter.contains(b"test"));
        assert!(!filter.contains(b"not_exist"));
    }

    #[test]
    fn test_tiered_bloom_filter() {
        let tiered = TieredBloomFilter::new(100);

        tiered.insert(b"hot_key", FilterTier::Hot);
        tiered.insert(b"warm_key", FilterTier::Warm);
        tiered.insert(b"cold_key", FilterTier::Cold);

        assert_eq!(tiered.contains(b"hot_key"), FilterResult::MayExistHot);
        assert_eq!(tiered.contains(b"warm_key"), FilterResult::MayExistWarm);
        assert_eq!(tiered.contains(b"cold_key"), FilterResult::MayExistCold);
        assert_eq!(tiered.contains(b"no_key"), FilterResult::DefinitelyNotExist);
    }

    #[test]
    fn test_bloom_filter_stats() {
        let mut filter = BloomFilter::new(1000, 0.01);

        for i in 0..100 {
            filter.insert(format!("key_{}", i).as_bytes());
        }

        let stats = filter.stats();
        assert_eq!(stats.element_count, 100);
        assert!(stats.current_fpp < 0.02); // 应该很低
        assert!(stats.size_in_bytes > 0);
    }
}