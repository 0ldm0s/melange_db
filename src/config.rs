use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fault_injection::{annotate, fallible};
use tempdir::TempDir;

use crate::{Db, smart_flush::SmartFlushConfig};

/// 压缩算法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompressionAlgorithm {
    /// Zstandard压缩 - 提供高压缩率，默认选择
    Zstd,
    /// LZ4压缩 - 提供更快的压缩/解压缩速度
    Lz4,
    /// 无压缩 - 提供最佳性能，适合低端设备
    None,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        // 明确的优先级处理：none > lz4 > zstd
        #[cfg(feature = "compression-none")]
        {
            Self::None
        }
        #[cfg(all(feature = "compression-lz4", not(feature = "compression-none")))]
        {
            Self::Lz4
        }
        #[cfg(all(feature = "compression-zstd", not(any(feature = "compression-none", feature = "compression-lz4"))))]
        {
            Self::Zstd
        }
        #[cfg(not(any(feature = "compression-zstd", feature = "compression-lz4", feature = "compression-none")))]
        {
            Self::None
        }
    }
}

impl CompressionAlgorithm {
    /// 检测当前编译时启用的压缩特性（用于调试）
    pub fn detect_enabled_features() -> Vec<&'static str> {
        let mut features = Vec::new();

        #[cfg(feature = "compression-zstd")]
        features.push("compression-zstd");

        #[cfg(feature = "compression-lz4")]
        features.push("compression-lz4");

        #[cfg(feature = "compression-none")]
        features.push("compression-none");

        features
    }

    /// 获取实际使用的压缩算法和原因（用于调试）
    pub fn get_active_algorithm_with_reason() -> (Self, &'static str) {
        let enabled_features = Self::detect_enabled_features();

        if enabled_features.contains(&"compression-none") {
            (Self::None, "compression-none特性已启用")
        } else if enabled_features.contains(&"compression-lz4") {
            (Self::Lz4, "compression-lz4特性已启用")
        } else if enabled_features.contains(&"compression-zstd") {
            (Self::Zstd, "compression-zstd特性已启用")
        } else {
            (Self::None, "未启用压缩特性，使用无压缩模式")
        }
    }

    /// 验证特性配置并返回警告信息
    pub fn validate_feature_config() -> Option<String> {
        let features = Self::detect_enabled_features();

        if features.len() > 1 {
            Some(format!(
                "警告：同时启用了多个压缩特性 {:?}，将使用优先级最高的特性。建议只启用一个压缩特性。",
                features
            ))
        } else {
            None
        }
    }
}

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
    /// 压缩算法选择。默认根据编译特性自动选择
    pub compression_algorithm: CompressionAlgorithm,
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
            compression_algorithm: CompressionAlgorithm::default(),
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
        (compression_algorithm, CompressionAlgorithm, "压缩算法选择。默认根据编译特性自动选择。"),
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