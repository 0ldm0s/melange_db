use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fault_injection::{annotate, fallible};
use tempdir::TempDir;

use crate::{Db, smart_flush::SmartFlushConfig};

macro_rules! builder {
    ($(($name:ident, $t:ty, $desc:expr)),*) => {
        $(
            #[doc=$desc]
            pub fn $name(mut self, to: $t) -> Self {
                self.$name = to;
                self
            }
        )*
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// 存储数据库的基础目录
    pub path: PathBuf,
    /// 缓存大小（字节）。默认为512mb
    pub cache_capacity_bytes: usize,
    /// 分配给扫描抗性入口缓存的缓存百分比
    pub entry_cache_percent: u8,
    /// 启动一个后台线程，每隔几毫秒将数据刷新到磁盘。默认为每200ms一次
    pub flush_every_ms: Option<usize>,
    /// 将数据写入磁盘时使用的zstd压缩级别。默认为3
    pub zstd_compression_level: i32,
    /// 这只为通过 `Config::tmp` 创建的对象设置为 `Some`，
    /// 并且在最后一个Arc删除时将删除存储目录
    pub tempdir_deleter: Option<Arc<TempDir>>,
    /// 0.0到1.0之间的浮点数，控制文件中可以存在多少碎片，
    /// 然后GC尝试重新压缩它
    pub target_heap_file_fill_ratio: f32,
    /// 大于此可配置值的值将作为单独的blob存储
    pub max_inline_value_threshold: usize,
    /// 增量序列化阈值（字节）。超过此大小的leaf节点将使用增量序列化
    pub incremental_serialization_threshold: usize,
    /// 异步flush线程数。默认为2
    pub flush_thread_count: usize,
    /// 缓存预热策略
    pub cache_warmup_strategy: CacheWarmupStrategy,
    /// 智能flush策略配置
    pub smart_flush_config: SmartFlushConfig,
}

#[derive(Debug, Clone)]
pub enum CacheWarmupStrategy {
    /// 无预热
    None,
    /// 预热最近访问的数据
    Recent,
    /// 预热热点数据
    Hot,
    /// 全部预热
    Full,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            path: "melange_db.default".into(),
            flush_every_ms: Some(200),
            cache_capacity_bytes: 512 * 1024 * 1024,
            entry_cache_percent: 20,
            zstd_compression_level: 3,
            tempdir_deleter: None,
            target_heap_file_fill_ratio: 0.9,
            max_inline_value_threshold: 4096,
            incremental_serialization_threshold: 8192,
            flush_thread_count: 2,
            cache_warmup_strategy: CacheWarmupStrategy::Recent,
            smart_flush_config: SmartFlushConfig::default(),
        }
    }
}

impl Config {
    /// 返回默认的 `Config`
    pub fn new() -> Config {
        Config::default()
    }

    /// 返回一个配置，其中 `path` 初始化为系统临时目录，
    /// 当此 `Config` 被删除时，该目录将被删除
    pub fn tmp() -> io::Result<Config> {
        let tempdir = fallible!(tempdir::TempDir::new("melange_db_tmp"));

        Ok(Config {
            path: tempdir.path().into(),
            tempdir_deleter: Some(Arc::new(tempdir)),
            ..Config::default()
        })
    }

    /// 设置数据库的路径（构建器）
    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Config {
        self.path = path.as_ref().to_path_buf();
        self
    }

    builder!(
        (flush_every_ms, Option<usize>, "启动一个后台线程，每隔几毫秒将数据刷新到磁盘。默认为每200ms一次。"),
        (cache_capacity_bytes, usize, "缓存大小（字节）。默认为512mb。"),
        (entry_cache_percent, u8, "分配给扫描抗性入口缓存的缓存百分比。"),
        (zstd_compression_level, i32, "将数据写入磁盘时使用的zstd压缩级别。默认为3。"),
        (target_heap_file_fill_ratio, f32, "0.0到1.0之间的浮点数，控制文件中可以存在多少碎片，然后GC尝试重新压缩它。"),
        (max_inline_value_threshold, usize, "大于此可配置值的值将作为单独的blob存储。"),
        (incremental_serialization_threshold, usize, "增量序列化阈值（字节）。超过此大小的leaf节点将使用增量序列化。"),
        (flush_thread_count, usize, "异步flush线程数。默认为2。"),
        (cache_warmup_strategy, CacheWarmupStrategy, "缓存预热策略。")
    );

    pub fn open<const LEAF_FANOUT: usize>(
        &self,
    ) -> io::Result<Db<LEAF_FANOUT>> {
        if LEAF_FANOUT < 3 {
            return Err(annotate!(io::Error::new(
                io::ErrorKind::Unsupported,
                "Db的LEAF_FANOUT const泛型必须为3或更大。"
            )));
        }
        Db::open_with_config(self)
    }
}