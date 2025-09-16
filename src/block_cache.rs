//! 高性能块缓存系统
//!
//! 此模块提供了针对数据库块读取优化的多级缓存系统，
//! 支持不同的缓存策略和智能淘汰算法。
//!
//! 主要特性：
//! - LRU淘汰策略
//! - 分级缓存（热/温/冷）
//! - 预取机制
//! - 自适应缓存大小
//! - 并发安全访问

use std::collections::{HashMap, LinkedList, VecDeque};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use parking_lot::RwLock as ParkingRwLock;
use serde::{Serialize, Deserialize};
use crate::debug_log;

/// 缓存块
#[derive(Debug, Clone)]
pub struct CacheBlock {
    /// 块数据
    pub data: Vec<u8>,
    /// 块ID
    pub block_id: u64,
    /// 访问次数
    pub access_count: u32,
    /// 最后访问时间
    pub last_access: Instant,
    /// 创建时间
    pub created_at: Instant,
    /// 块大小
    pub size: usize,
    /// 访问模式统计
    pub access_pattern: AccessPattern,
}

/// 访问模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,  // 顺序访问
    Random,     // 随机访问
    Unknown,    // 未知模式
}

/// 缓存淘汰策略
#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    LRU,           // 最近最少使用
    LFU,           // 最不经常使用
    ARC,           // 自适应替换缓存
    SizeAware,     // 大小感知
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 总缓存大小（字节）
    pub max_size: usize,
    /// 块大小（字节）
    pub block_size: usize,
    /// 淘汰策略
    pub eviction_policy: EvictionPolicy,
    /// 启用预取
    pub enable_prefetch: bool,
    /// 预取窗口大小
    pub prefetch_window: usize,
    /// 启用压缩
    pub enable_compression: bool,
    /// 压缩阈值（字节）
    pub compression_threshold: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 256 * 1024 * 1024, // 256MB
            block_size: 4096,            // 4KB
            eviction_policy: EvictionPolicy::ARC,
            enable_prefetch: true,
            prefetch_window: 4,
            enable_compression: true,
            compression_threshold: 1024, // 1KB
        }
    }
}

/// LRU缓存节点
#[derive(Debug)]
struct LruNode {
    block: CacheBlock,
    prev: Option<*mut LruNode>,
    next: Option<*mut LruNode>,
}

/// LRU缓存实现
#[derive(Debug)]
struct LruCache {
    /// 哈希表：block_id -> 节点
    map: HashMap<u64, Box<LruNode>>,
    /// 双向链表头部
    head: Option<*mut LruNode>,
    /// 双向链表尾部
    tail: Option<*mut LruNode>,
    /// 当前大小
    current_size: usize,
    /// 最大大小
    max_size: usize,
}

impl LruCache {
    fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::new(),
            head: None,
            tail: None,
            current_size: 0,
            max_size,
        }
    }

    fn get(&mut self, block_id: u64) -> Option<CacheBlock> {
        if self.map.contains_key(&block_id) {
            let node = self.map.remove(&block_id).unwrap();
            let block = node.block.clone();

            // 重新插入以移动到头部，但不增加大小
            let block_size = block.size;
            self.current_size = self.current_size.saturating_sub(block_size);
            self.put_without_size_check(block.clone());

            Some(block)
        } else {
            None
        }
    }

    fn put(&mut self, block: CacheBlock) -> Option<CacheBlock> {
        let block_size = block.size;

        // 如果块已存在，移除旧块
        if self.map.contains_key(&block.block_id) {
            let old_node = self.map.remove(&block.block_id).unwrap();
            self.current_size = self.current_size.saturating_sub(old_node.block.size);
            return Some(old_node.block);
        }

        // 检查是否需要淘汰
        while self.current_size + block_size > self.max_size {
            if let Some(evicted) = self.evict() {
                self.current_size -= evicted.size;
            } else {
                break;
            }
        }

        // 创建新节点
        let mut node = Box::new(LruNode {
            block: block.clone(),
            prev: None,
            next: self.head,
        });

        let node_ptr = node.as_mut() as *mut LruNode;

        // 更新头部
        if let Some(head) = self.head {
            unsafe {
                (*head).prev = Some(node_ptr);
            }
        }
        self.head = Some(node_ptr);

        // 如果是第一个节点，设置尾部
        if self.tail.is_none() {
            self.tail = Some(node_ptr);
        }

        // 插入哈希表
        self.map.insert(block.block_id, node);
        self.current_size += block_size;

        None
    }

    fn put_without_size_check(&mut self, block: CacheBlock) {
        let block_size = block.size;

        // 创建新节点
        let mut node = Box::new(LruNode {
            block: block.clone(),
            prev: None,
            next: self.head,
        });

        let node_ptr = node.as_mut() as *mut LruNode;

        // 更新头部
        if let Some(head) = self.head {
            unsafe {
                (*head).prev = Some(node_ptr);
            }
        }
        self.head = Some(node_ptr);

        // 如果是第一个节点，设置尾部
        if self.tail.is_none() {
            self.tail = Some(node_ptr);
        }

        // 插入哈希表
        self.map.insert(block.block_id, node);
        self.current_size += block_size;
    }

    fn move_to_head(&mut self, node: &mut Box<LruNode>) {
        let node_ptr = node.as_mut() as *mut LruNode;

        // 如果已经在头部，无需移动
        if let Some(head) = self.head {
            if head == node_ptr {
                return;
            }
        }

        // 从当前位置移除
        if let Some(prev) = node.prev {
            unsafe {
                (*prev).next = node.next;
            }
        } else {
            // 当前是头部，无需特殊处理
            return;
        }

        if let Some(next) = node.next {
            unsafe {
                (*next).prev = node.prev;
            }
        } else {
            // 当前是尾部，更新尾部
            self.tail = node.prev;
        }

        // 移动到头部
        node.prev = None;
        node.next = self.head;

        if let Some(head) = self.head {
            unsafe {
                (*head).prev = Some(node_ptr);
            }
        }
        self.head = Some(node_ptr);
    }

    fn evict(&mut self) -> Option<CacheBlock> {
        if let Some(tail_ptr) = self.tail {
            unsafe {
                let tail = &mut *tail_ptr;

                // 更新尾部指针
                self.tail = tail.prev;

                if let Some(prev) = tail.prev {
                    (*prev).next = None;
                } else {
                    // 如果没有前驱，说明只有一个节点
                    self.head = None;
                }

                // 从哈希表中移除
                let block_id = tail.block.block_id;
                let block = self.map.remove(&block_id).unwrap().block;

                Some(block)
            }
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.head = None;
        self.tail = None;
        self.current_size = 0;
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn size(&self) -> usize {
        self.current_size
    }
}

/// 分级块缓存
#[derive(Debug)]
pub struct TieredBlockCache {
    /// 热缓存（最近访问）
    hot_cache: Arc<ParkingRwLock<LruCache>>,
    /// 温缓存（中等频率）
    warm_cache: Arc<ParkingRwLock<LruCache>>,
    /// 冷缓存（较少访问）
    cold_cache: Arc<ParkingRwLock<LruCache>>,
    /// 配置
    config: CacheConfig,
    /// 预取队列
    prefetch_queue: Arc<Mutex<VecDeque<u64>>>,
    /// 访问模式检测
    access_patterns: Arc<RwLock<HashMap<u64, AccessPattern>>>,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

/// 缓存统计信息（内部实现细节）
#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub prefetch_hits: u64,
    pub prefetch_misses: u64,
    pub hot_hits: u64,
    pub warm_hits: u64,
    pub cold_hits: u64,
    pub total_bytes_served: u64,
    pub compression_ratio: f64,
}

impl TieredBlockCache {
    pub fn new(config: CacheConfig) -> Self {
        let hot_size = (config.max_size as f64 * 0.1) as usize;  // 10% 热缓存
        let warm_size = (config.max_size as f64 * 0.3) as usize; // 30% 温缓存
        let cold_size = (config.max_size as f64 * 0.6) as usize; // 60% 冷缓存

        debug_log!("创建分级块缓存: 热={}, 温={}, 冷={}", hot_size, warm_size, cold_size);

        Self {
            hot_cache: Arc::new(ParkingRwLock::new(LruCache::new(hot_size))),
            warm_cache: Arc::new(ParkingRwLock::new(LruCache::new(warm_size))),
            cold_cache: Arc::new(ParkingRwLock::new(LruCache::new(cold_size))),
            config,
            prefetch_queue: Arc::new(Mutex::new(VecDeque::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// 获取缓存块
    pub fn get(&self, block_id: u64) -> Option<CacheBlock> {
        // 先尝试热缓存
        if let Some(block) = self.hot_cache.write().get(block_id) {
            self.update_stats(true, CacheTier::Hot);
            return Some(block);
        }

        // 再尝试温缓存
        if let Some(block) = self.warm_cache.write().get(block_id) {
            self.update_stats(true, CacheTier::Warm);
            // 提升到热缓存
            self.promote_to_hot(block.clone());
            return Some(block);
        }

        // 最后尝试冷缓存
        if let Some(block) = self.cold_cache.write().get(block_id) {
            self.update_stats(true, CacheTier::Cold);
            // 提升到温缓存
            self.promote_to_warm(block.clone());
            return Some(block);
        }

        // 缓存未命中
        self.update_stats(false, CacheTier::Cold);
        None
    }

    /// 存储缓存块
    pub fn put(&self, mut block: CacheBlock) {
        // 更新访问模式
        self.update_access_pattern(block.block_id);

        // 压缩大块
        if self.config.enable_compression && block.size > self.config.compression_threshold {
            if let Ok(compressed) = self.compress_block(&block) {
                block.data = compressed;
                block.size = block.data.len();
            }
        }

        // 存储到温缓存（新数据通常有一定的访问频率）
        self.warm_cache.write().put(block.clone());

        // 触发预取
        if self.config.enable_prefetch {
            self.trigger_prefetch(block.block_id);
        }
    }

    /// 提升块到热缓存
    fn promote_to_hot(&self, block: CacheBlock) {
        self.hot_cache.write().put(block);
    }

    /// 提升块到温缓存
    fn promote_to_warm(&self, block: CacheBlock) {
        self.warm_cache.write().put(block);
    }

    /// 触发预取
    fn trigger_prefetch(&self, current_block_id: u64) {
        let mut queue = self.prefetch_queue.lock().unwrap();

        // 预取后续块
        for i in 1..=self.config.prefetch_window {
            let next_block_id = current_block_id + i as u64;
            if !queue.contains(&next_block_id) {
                queue.push_back(next_block_id);
            }
        }
    }

    /// 获取预取任务
    pub fn get_prefetch_task(&self) -> Option<u64> {
        let mut queue = self.prefetch_queue.lock().unwrap();
        queue.pop_front()
    }

    /// 更新访问模式
    fn update_access_pattern(&self, block_id: u64) {
        let mut patterns = self.access_patterns.write().unwrap();
        let pattern = patterns.entry(block_id).or_insert(AccessPattern::Unknown);

        // 简单的访问模式检测逻辑
        // 实际实现中可能需要更复杂的算法
        *pattern = match pattern {
            AccessPattern::Unknown => AccessPattern::Sequential,
            AccessPattern::Sequential => AccessPattern::Sequential,
            AccessPattern::Random => AccessPattern::Random,
        };
    }

    /// 压缩块数据
    fn compress_block(&self, block: &CacheBlock) -> Result<Vec<u8>, String> {
        use zstd::bulk::compress;

        match compress(&block.data, 3) { // 压缩级别3
            Ok(compressed) => {
                if compressed.len() < block.data.len() {
                    Ok(compressed)
                } else {
                    Err("压缩后没有节省空间".to_string())
                }
            }
            Err(e) => Err(format!("压缩失败: {}", e)),
        }
    }

    /// 更新统计信息
    fn update_stats(&self, hit: bool, tier: CacheTier) {
        let mut stats = self.stats.write().unwrap();

        if hit {
            stats.hits += 1;
            match tier {
                CacheTier::Hot => stats.hot_hits += 1,
                CacheTier::Warm => stats.warm_hits += 1,
                CacheTier::Cold => stats.cold_hits += 1,
            }
        } else {
            stats.misses += 1;
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.hot_cache.write().clear();
        self.warm_cache.write().clear();
        self.cold_cache.write().clear();
        self.prefetch_queue.lock().unwrap().clear();
        self.access_patterns.write().unwrap().clear();
    }

    /// 获取缓存大小信息
    pub fn size_info(&self) -> CacheSizeInfo {
        CacheSizeInfo {
            hot_size: self.hot_cache.read().size(),
            warm_size: self.warm_cache.read().size(),
            cold_size: self.cold_cache.read().size(),
            hot_blocks: self.hot_cache.read().len(),
            warm_blocks: self.warm_cache.read().len(),
            cold_blocks: self.cold_cache.read().len(),
        }
    }
}

/// 缓存层级
#[derive(Debug, Clone, Copy)]
enum CacheTier {
    Hot,
    Warm,
    Cold,
}

/// 缓存大小信息
#[derive(Debug, Clone)]
pub struct CacheSizeInfo {
    pub hot_size: usize,
    pub warm_size: usize,
    pub cold_size: usize,
    pub hot_blocks: usize,
    pub warm_blocks: usize,
    pub cold_blocks: usize,
}

/// 智能缓存管理器
#[derive(Debug)]
pub struct CacheManager {
    block_cache: Arc<TieredBlockCache>,
    config: CacheConfig,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            block_cache: Arc::new(TieredBlockCache::new(config.clone())),
            config,
        }
    }

    /// 读取块数据
    pub fn read_block(&self, block_id: u64) -> Option<CacheBlock> {
        // 尝试从缓存读取
        if let Some(block) = self.block_cache.get(block_id) {
            return Some(block);
        }

        // 缓存未命中，需要从磁盘读取
        // 这里应该调用实际的磁盘读取函数
        // 暂时返回None，实际实现中需要补充
        None
    }

    /// 写入块数据
    pub fn write_block(&self, block_id: u64, data: Vec<u8>) {
        let size = data.len();
        let block = CacheBlock {
            data,
            block_id,
            access_count: 1,
            last_access: Instant::now(),
            created_at: Instant::now(),
            size,
            access_pattern: AccessPattern::Unknown,
        };

        self.block_cache.put(block);
    }

    /// 批量预取
    pub fn prefetch_blocks(&self, block_ids: &[u64]) {
        for &block_id in block_ids {
            // 如果缓存中没有，则触发预取
            if self.block_cache.get(block_id).is_none() {
                self.block_cache.trigger_prefetch(block_id);
            }
        }
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        self.block_cache.stats()
    }

    /// 获取缓存大小信息
    pub fn size_info(&self) -> CacheSizeInfo {
        self.block_cache.size_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic() {
        let mut cache = LruCache::new(1024);

        // 插入测试块
        let block1 = CacheBlock {
            data: vec![1u8; 100],
            block_id: 1,
            access_count: 1,
            last_access: Instant::now(),
            created_at: Instant::now(),
            size: 100,
            access_pattern: AccessPattern::Unknown,
        };

        let block2 = CacheBlock {
            data: vec![2u8; 200],
            block_id: 2,
            access_count: 1,
            last_access: Instant::now(),
            created_at: Instant::now(),
            size: 200,
            access_pattern: AccessPattern::Unknown,
        };

        assert!(cache.put(block1).is_none());
        assert!(cache.put(block2).is_none());

        // 测试读取
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_none());

        // 测试大小
        assert_eq!(cache.size(), 300);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_tiered_block_cache() {
        let config = CacheConfig::default();
        let cache = TieredBlockCache::new(config);

        let block = CacheBlock {
            data: vec![1u8; 100],
            block_id: 1,
            access_count: 1,
            last_access: Instant::now(),
            created_at: Instant::now(),
            size: 100,
            access_pattern: AccessPattern::Unknown,
        };

        // 测试插入和读取
        cache.put(block.clone());
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());

        // 测试统计信息
        let stats = cache.stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_manager() {
        let config = CacheConfig::default();
        let manager = CacheManager::new(config);

        let block_id = 1;
        let data = vec![1u8; 100];

        // 测试写入
        manager.write_block(block_id, data.clone());

        // 测试读取
        let cached_block = manager.read_block(block_id);
        assert!(cached_block.is_some());
        assert_eq!(cached_block.unwrap().data, data);
    }
}