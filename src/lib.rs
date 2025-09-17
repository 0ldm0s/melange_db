// melange_db - 高性能嵌入式数据库
// 在sled架构基础上进行了深度性能优化，目标超越RocksDB性能

//! `melange_db` 是一个高性能嵌入式数据库，
//! 在sled架构基础上进行了深度性能优化，目标超越RocksDB性能。
//!
//! 主要优化包括：
//! - 增量序列化减少IO开销
//! - 改进的缓存策略
//! - 优化的flush机制
//! - 更高效的内存管理

#[cfg(feature = "for-internal-testing-only")]
mod block_checker;
pub mod block_cache;
pub mod bloom_filter;
pub mod smart_flush;
mod config;
mod db;
mod flush_epoch;
mod heap;
mod id_allocator;
mod leaf;
mod logging;
mod metadata_store;
mod object_cache;
mod object_location_mapper;
pub mod platform_utils;
pub mod simd_optimized;
mod tree;

#[cfg(any(
    feature = "testing-shred-allocator",
    feature = "testing-count-allocator"
))]
pub mod alloc;

#[cfg(feature = "for-internal-testing-only")]
mod event_verifier;

#[inline]
fn debug_delay() {
    #[cfg(debug_assertions)]
    {
        let rand =
            std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap().as_nanos();

        if rand % 128 > 100 {
            for _ in 0..rand % 16 {
                std::thread::yield_now();
            }
        }
    }
}

pub use crate::config::{Config, CacheWarmupStrategy, CompressionAlgorithm};
pub use crate::db::Db;
pub use crate::tree::{Batch, Iter, Tree};

// 内部优化实现细节，不应暴露给用户
#[doc(hidden)]
pub use crate::block_cache::{CacheManager, CacheConfig, AccessPattern};
#[doc(hidden)]
pub use crate::bloom_filter::{BloomFilter, ConcurrentBloomFilter, TieredBloomFilter, FilterTier};
#[doc(hidden)]
pub use crate::simd_optimized::{SimdComparator, KeyComparator};
pub use inline_array::InlineArray;

const NAME_MAPPING_COLLECTION_ID: CollectionId = CollectionId(0);
const DEFAULT_COLLECTION_ID: CollectionId = CollectionId(1);
const INDEX_FANOUT: usize = 64;
const EBR_LOCAL_GC_BUFFER_SIZE: usize = 128;

use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::ops::Bound;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::flush_epoch::{
    FlushEpoch, FlushEpochGuard, FlushEpochTracker, FlushInvariants,
};
use crate::heap::{
    HeapStats, ObjectRecovery, SlabAddress, Update, WriteBatchStats,
};
use crate::id_allocator::{Allocator, DeferredFree};
use crate::leaf::Leaf;

// 这些是公开的，以便在外部二进制文件中进行崩溃测试
// 它们被隐藏是因为没有关于其API稳定性或功能的保证
#[doc(hidden)]
pub use crate::heap::{Heap, HeapRecovery};
#[doc(hidden)]
pub use crate::metadata_store::MetadataStore;
#[doc(hidden)]
pub use crate::object_cache::{CacheStats, Dirty, FlushStats, ObjectCache};

/// 使用默认配置在指定路径打开一个 `Db`
/// 这将在指定路径创建一个新的存储目录（如果它不存在）
/// 您可以使用 `Db::was_recovered` 方法来确定数据库是否从之前的实例中恢复
pub fn open<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Db> {
    Config::new().path(path).open()
}

#[derive(Debug, Copy, Clone)]
pub struct Stats {
    pub cache: CacheStats,
}

/// 比较并交换结果
///
/// 它返回 `Ok(Ok(()))` 如果操作成功完成
///     - `Ok(Err(CompareAndSwapError(current, proposed)))` 如果操作失败
///       无法设置新值。`CompareAndSwapError` 包含当前和提议的值。
///     - `Err(Error::Unsupported)` 如果数据库以只读模式打开。
pub type CompareAndSwapResult = std::io::Result<
    std::result::Result<CompareAndSwapSuccess, CompareAndSwapError>,
>;

type Index<const LEAF_FANOUT: usize> = concurrent_map::ConcurrentMap<
    InlineArray,
    Object<LEAF_FANOUT>,
    INDEX_FANOUT,
    EBR_LOCAL_GC_BUFFER_SIZE,
>;

/// 比较并交换错误
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompareAndSwapError {
    /// 导致您的CAS失败的当前值
    pub current: Option<InlineArray>,
    /// 返回的未成功提出的值
    pub proposed: Option<InlineArray>,
}

/// 比较并交换成功
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompareAndSwapSuccess {
    /// 成功安装的当前值
    pub new_value: Option<InlineArray>,
    /// 之前存储的返回值
    pub previous_value: Option<InlineArray>,
}

impl std::fmt::Display for CompareAndSwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compare and swap conflict")
    }
}

impl std::error::Error for CompareAndSwapError {}

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
pub struct ObjectId(NonZeroU64);

impl ObjectId {
    fn new(from: u64) -> Option<ObjectId> {
        NonZeroU64::new(from).map(ObjectId)
    }
}

impl std::ops::Deref for ObjectId {
    type Target = u64;

    fn deref(&self) -> &u64 {
        let self_ref: &NonZeroU64 = &self.0;

        // NonZeroU64 是 repr(transparent) 包装了一个 u64
        // 所以它保证匹配二进制布局。这使得
        // 可以安全地将一个引用转换为另一个引用
        let self_ptr: *const NonZeroU64 = self_ref as *const _;
        let reference: *const u64 = self_ptr as *const u64;

        unsafe { &*reference }
    }
}

impl concurrent_map::Minimum for ObjectId {
    const MIN: ObjectId = ObjectId(NonZeroU64::MIN);
}

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
pub struct CollectionId(u64);

impl concurrent_map::Minimum for CollectionId {
    const MIN: CollectionId = CollectionId(u64::MIN);
}

#[derive(Debug, Clone)]
struct CacheBox<const LEAF_FANOUT: usize> {
    leaf: Option<Box<Leaf<LEAF_FANOUT>>>,
    #[allow(unused)]
    logged_index: BTreeMap<InlineArray, LogValue>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct LogValue {
    location: SlabAddress,
    value: Option<InlineArray>,
}

#[derive(Debug, Clone)]
pub struct Object<const LEAF_FANOUT: usize> {
    object_id: ObjectId,
    collection_id: CollectionId,
    low_key: InlineArray,
    inner: Arc<RwLock<CacheBox<LEAF_FANOUT>>>,
}

impl<const LEAF_FANOUT: usize> PartialEq for Object<LEAF_FANOUT> {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

/// 存储在 `Db` 和 `Tree` 的 Arc 中，
/// 所以当最后一个"高级"结构被删除时，
/// flusher 线程被清理
struct ShutdownDropper<const LEAF_FANOUT: usize> {
    shutdown_sender: parking_lot::Mutex<
        std::sync::mpsc::Sender<std::sync::mpsc::Sender<()>>,
    >,
    cache: parking_lot::Mutex<object_cache::ObjectCache<LEAF_FANOUT>>,
}

impl<const LEAF_FANOUT: usize> Drop for ShutdownDropper<LEAF_FANOUT> {
    fn drop(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        debug_log!("sending shutdown signal to flusher");
        if self.shutdown_sender.lock().send(tx).is_ok() {
            if let Err(e) = rx.recv() {
                error_log!("failed to shut down flusher thread: {:?}", e);
            } else {
                debug_log!("flush thread successfully terminated");
            }
        } else {
            debug_log!(
                "failed to shut down flusher, manually flushing ObjectCache"
            );
            let cache = self.cache.lock();
            if let Err(e) = cache.flush() {
                error_log!(
                    "Db flusher encountered error while flushing: {:?}",
                    e
                );
                cache.set_error(&e);
            }
        }
    }
}

fn map_bound<T, U, F: FnOnce(T) -> U>(bound: Bound<T>, f: F) -> Bound<U> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(x) => Bound::Included(f(x)),
        Bound::Excluded(x) => Bound::Excluded(f(x)),
    }
}

const fn _assert_public_types_send_sync() {
    use std::fmt::Debug;

    const fn _assert_send<S: Send + Clone + Debug>() {}

    const fn _assert_send_sync<S: Send + Sync + Clone + Debug>() {}

    _assert_send::<Db>();

    _assert_send_sync::<Batch>();
    _assert_send_sync::<InlineArray>();
    _assert_send_sync::<Config>();
    _assert_send_sync::<CompareAndSwapSuccess>();
    _assert_send_sync::<CompareAndSwapError>();
}
