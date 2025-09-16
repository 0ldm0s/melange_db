use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::*;
use crate::{debug_log, trace_log, warn_log, error_log, info_log, smart_flush::{SmartFlushScheduler, SmartFlushConfig}};

/// melange_db - 高性能嵌入式数据库
///
/// 在 sled 架构基础上进行了深度性能优化，目标是超越 RocksDB 性能
/// 主要优化包括：增量序列化、改进的缓存策略、优化的flush机制等
///
/// `Db` 和 `Tree` 有一个 `LEAF_FANOUT` const 泛型参数。这个参数允许用户在性能与效率之间进行权衡。
/// 默认值 `1024` 使键和值在存储到磁盘时更有效地压缩，
/// 但对于大于内存的随机工作负载，将 `LEAF_FANOUT` 降低到 `16` 到 `256` 之间可能是有利的，
/// 具体取决于您的效率要求。较低的值还会减少频繁访问数据的竞争。
/// 创建数据库后无法更改此值。
///
/// 这是一个优化版本，专注于提供更好的性能和稳定性。
///
/// 注意 `Db` 为默认的 `Tree`（melange_db 的命名空间/键空间/桶版本）
/// 实现了 `Deref`，但您可以使用 `Db::open_tree` 创建和使用其他的。
#[derive(Clone)]
pub struct Db<const LEAF_FANOUT: usize = 1024> {
    config: Config,
    _shutdown_dropper: Arc<ShutdownDropper<LEAF_FANOUT>>,
    cache: ObjectCache<LEAF_FANOUT>,
    trees: Arc<Mutex<HashMap<CollectionId, Tree<LEAF_FANOUT>>>>,
    collection_id_allocator: Arc<Allocator>,
    collection_name_mapping: Tree<LEAF_FANOUT>,
    default_tree: Tree<LEAF_FANOUT>,
    was_recovered: bool,
}

impl<const LEAF_FANOUT: usize> std::ops::Deref for Db<LEAF_FANOUT> {
    type Target = Tree<LEAF_FANOUT>;
    fn deref(&self) -> &Tree<LEAF_FANOUT> {
        &self.default_tree
    }
}

impl<const LEAF_FANOUT: usize> IntoIterator for &Db<LEAF_FANOUT> {
    type Item = io::Result<(InlineArray, InlineArray)>;
    type IntoIter = crate::Iter<LEAF_FANOUT>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<const LEAF_FANOUT: usize> fmt::Debug for Db<LEAF_FANOUT> {
    fn fmt(&self, w: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alternate = w.alternate();

        let mut debug_struct = w.debug_struct(&format!("Db<{}>", LEAF_FANOUT));

        if alternate {
            debug_struct
                .field("global_error", &self.check_error())
                .field(
                    "data",
                    &format!("{:?}", self.iter().collect::<Vec<_>>()),
                )
                .finish()
        } else {
            debug_struct.field("global_error", &self.check_error()).finish()
        }
    }
}

// 线程安全实现：Db可以在多线程间安全共享
unsafe impl<const LEAF_FANOUT: usize> Send for Db<LEAF_FANOUT> {}
unsafe impl<const LEAF_FANOUT: usize> Sync for Db<LEAF_FANOUT> {}

fn flusher<const LEAF_FANOUT: usize>(
    cache: ObjectCache<LEAF_FANOUT>,
    shutdown_signal: mpsc::Receiver<mpsc::Sender<()>>,
    flush_every_ms: usize,
) {
    let interval = Duration::from_millis(flush_every_ms as _);
    let mut last_flush_duration = Duration::default();

    let flush = || {
        let flush_res_res = std::panic::catch_unwind(|| cache.flush());
        match flush_res_res {
            Ok(Ok(_)) => {
                // 不中止。
                return;
            }
            Ok(Err(flush_failure)) => {
                error_log!(
                    "Db flusher 在刷新时遇到错误: {:?}",
                    flush_failure
                );
                cache.set_error(&flush_failure);
            }
            Err(panicked) => {
                error_log!(
                    "Db flusher 在刷新时发生恐慌: {:?}",
                    panicked
                );
                cache.set_error(&io::Error::other(
                    "Db flusher 在刷新时发生恐慌".to_string(),
                ));
            }
        }
        std::process::abort();
    };

    loop {
        let recv_timeout = interval
            .saturating_sub(last_flush_duration)
            .max(Duration::from_millis(1));
        if let Ok(shutdown_sender) = shutdown_signal.recv_timeout(recv_timeout)
        {
            flush();

            // 这可能是不必要的，但如果引入了会触发它的严重错误，
            // 它将避免问题
            cache.set_error(&io::Error::other(
                "系统已关闭".to_string(),
            ));

            assert!(cache.is_clean());

            drop(cache);

            if let Err(e) = shutdown_sender.send(()) {
                error_log!(
                    "Db flusher 无法向请求者确认关闭: {e:?}"
                );
            }
            debug_log!(
                "flush 线程在向请求者发出信号后终止"
            );
            return;
        }

        let before_flush = Instant::now();

        flush();

        last_flush_duration = before_flush.elapsed();
    }
}

impl<const LEAF_FANOUT: usize> Drop for Db<LEAF_FANOUT> {
    fn drop(&mut self) {
        if self.config.flush_every_ms.is_none() {
            if let Err(e) = self.flush() {
                error_log!("在 Drop 时刷新 Db 失败: {e:?}");
            }
        } else {
            // 否则，预期 flusher 线程将在关闭最后的 Db/Tree 实例时刷新
        }
    }
}

impl<const LEAF_FANOUT: usize> Db<LEAF_FANOUT> {
    #[cfg(feature = "for-internal-testing-only")]
    fn validate(&self) -> io::Result<()> {
        // 对于每个树，遍历索引，读取节点并断言低键匹配
        // 并断言这是我们第一次看到节点 ID

        let mut ever_seen = std::collections::HashSet::new();
        let before = std::time::Instant::now();

        #[cfg(feature = "for-internal-testing-only")]
        let _b0 = crate::block_checker::track_blocks();

        for (_cid, tree) in self.trees.lock().iter() {
            let mut hi_none_count = 0;
            let mut last_hi = None;
            for (low, node) in tree.index.iter() {
                // 确保我们没有在树之间重用 object_id
                assert!(ever_seen.insert(node.object_id));

                let (read_low, node_mu, read_node) =
                    tree.page_in(&low, self.cache.current_flush_epoch())?;

                assert_eq!(read_node.object_id, node.object_id);
                assert_eq!(node_mu.leaf.as_ref().unwrap().lo, low);
                assert_eq!(read_low, low);

                if let Some(hi) = &last_hi {
                    assert_eq!(hi, &node_mu.leaf.as_ref().unwrap().lo);
                }

                if let Some(hi) = &node_mu.leaf.as_ref().unwrap().hi {
                    last_hi = Some(hi.clone());
                } else {
                    assert_eq!(hi_none_count, 0);
                    hi_none_count += 1;
                }
            }
            // 每个树应该只有一个叶子节点没有最大 hi 键
            assert_eq!(hi_none_count, 1);
        }

        debug_log!(
            "{} 个叶子节点在 {} 微秒后看起来正常",
            ever_seen.len(),
            before.elapsed().as_micros()
        );

        Ok(())
    }

    pub fn stats(&self) -> Stats {
        Stats { cache: self.cache.stats() }
    }

    pub fn size_on_disk(&self) -> io::Result<u64> {
        use std::fs::read_dir;

        fn recurse(mut dir: std::fs::ReadDir) -> io::Result<u64> {
            dir.try_fold(0, |acc, file| {
                let file = file?;
                let size = match file.metadata()? {
                    data if data.is_dir() => recurse(read_dir(file.path())?)?,
                    data => data.len(),
                };
                Ok(acc + size)
            })
        }

        recurse(read_dir(&self.cache.config.path)?)
    }

    /// 如果数据库是从之前的进程恢复的，则返回 `true`。
    /// 请注意，数据库状态仅在最后一次调用 `flush` 时保证存在！
    /// 否则，如果 `Config.sync_every_ms` 配置选项设置为
    /// `Some(number_of_ms_between_syncs)` 或如果 IO 缓冲区在被轮换之前填满容量，
    /// 状态会定期同步到磁盘。
    pub fn was_recovered(&self) -> bool {
        self.was_recovered
    }

    pub fn open_with_config(config: &Config) -> io::Result<Db<LEAF_FANOUT>> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let (cache, indices, was_recovered) = ObjectCache::recover(config)?;

        let _shutdown_dropper = Arc::new(ShutdownDropper {
            shutdown_sender: Mutex::new(shutdown_tx),
            cache: Mutex::new(cache.clone()),
        });

        let mut allocated_collection_ids = fnv::FnvHashSet::default();

        let mut trees: HashMap<CollectionId, Tree<LEAF_FANOUT>> = indices
            .into_iter()
            .map(|(collection_id, index)| {
                assert!(
                    allocated_collection_ids.insert(collection_id.0),
                    "allocated_collection_ids 已经包含 {:?}",
                    collection_id
                );
                (
                    collection_id,
                    Tree::new(
                        collection_id,
                        cache.clone(),
                        index,
                        _shutdown_dropper.clone(),
                    ),
                )
            })
            .collect();

        let collection_name_mapping =
            trees.get(&NAME_MAPPING_COLLECTION_ID).unwrap().clone();

        let default_tree = trees.get(&DEFAULT_COLLECTION_ID).unwrap().clone();

        for kv_res in collection_name_mapping.iter() {
            let (_collection_name, collection_id_buf) = kv_res.unwrap();
            let collection_id = CollectionId(u64::from_le_bytes(
                collection_id_buf.as_ref().try_into().unwrap(),
            ));

            if trees.contains_key(&collection_id) {
                continue;
            }

            // 需要为空集合初始化树叶子节点

            assert!(
                allocated_collection_ids.insert(collection_id.0),
                "allocated_collection_ids 已经包含 {:?}",
                collection_id
            );

            let initial_low_key = InlineArray::default();

            let empty_node = cache.allocate_default_node(collection_id);

            let index = Index::default();

            assert!(index.insert(initial_low_key, empty_node).is_none());

            let tree = Tree::new(
                collection_id,
                cache.clone(),
                index,
                _shutdown_dropper.clone(),
            );

            trees.insert(collection_id, tree);
        }

        let collection_id_allocator =
            Arc::new(Allocator::from_allocated(&allocated_collection_ids));

        assert_eq!(collection_name_mapping.len()? + 2, trees.len());

        let ret = Db {
            config: config.clone(),
            cache: cache.clone(),
            default_tree,
            collection_name_mapping,
            collection_id_allocator,
            trees: Arc::new(Mutex::new(trees)),
            _shutdown_dropper,
            was_recovered,
        };

        #[cfg(feature = "for-internal-testing-only")]
        ret.validate()?;

        if let Some(flush_every_ms) = ret.cache.config.flush_every_ms {
            let smart_config = ret.cache.config.smart_flush_config.clone();

            if smart_config.enabled {
                // 使用智能flusher
                let spawn_res = std::thread::Builder::new()
                    .name("melange_db_smart_flusher".into())
                    .spawn(move || smart_flusher(cache, shutdown_rx, smart_config));

                if let Err(e) = spawn_res {
                    return Err(io::Error::other(format!(
                        "无法为 melange_db 数据库生成智能 flusher 线程: {:?}",
                        e
                    )));
                }
                info_log!("已启动智能flusher线程");
            } else {
                // 使用传统固定间隔flusher
                let spawn_res = std::thread::Builder::new()
                    .name("melange_db_flusher".into())
                    .spawn(move || flusher(cache, shutdown_rx, flush_every_ms));

                if let Err(e) = spawn_res {
                    return Err(io::Error::other(format!(
                        "无法为 melange_db 数据库生成 flusher 线程: {:?}",
                        e
                    )));
                }
                info_log!("已启动传统flusher线程");
            }
        }
        Ok(ret)
    }

    /// `Db` 中所有集合的数据库导出方法，用于 melange_db 版本升级。
    /// 可以与运行较新版本的数据库上的 `import` 方法结合使用。
    ///
    /// # Panics
    ///
    /// 如果在尝试执行导出时发生任何 IO 问题，则会发生恐慌。
    ///
    /// # Examples
    ///
    /// 如果您想从一个版本的 melange_db 迁移到另一个版本，
    /// 您需要通过使用版本重命名引入两个版本：
    ///
    /// `Cargo.toml`:
    ///
    /// ```toml
    /// [dependencies]
    /// melange_db = "0.32"
    /// old_melange_db = { version = "0.31", package = "melange_db" }
    /// ```
    ///
    /// 在您的代码中，请记住旧版本的 melange_db 可能有不同的打开方式
    /// 比当前的 `melange_db::open` 方法：
    ///
    /// ```
    /// # use melange_db as old_melange_db;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let old = old_melange_db::open("my_old_db_export")?;
    ///
    /// // 可能是不同版本的 melange_db，
    /// // 导出类型是版本不可知的。
    /// let new = melange_db::open("my_new_db_export")?;
    ///
    /// let export = old.export();
    /// new.import(export);
    ///
    /// assert_eq!(old.checksum()?, new.checksum()?);
    /// # drop(old);
    /// # drop(new);
    /// # let _ = std::fs::remove_dir_all("my_old_db_export");
    /// # let _ = std::fs::remove_dir_all("my_new_db_export");
    /// # Ok(()) }
    /// ```
    pub fn export(
        &self,
    ) -> Vec<(
        CollectionType,
        CollectionName,
        impl Iterator<Item = Vec<Vec<u8>>> + '_,
    )> {
        let trees = self.trees.lock();

        let mut ret = vec![];

        for kv_res in self.collection_name_mapping.iter() {
            let (collection_name, collection_id_buf) = kv_res.unwrap();
            let collection_id = CollectionId(u64::from_le_bytes(
                collection_id_buf.as_ref().try_into().unwrap(),
            ));
            let tree = trees.get(&collection_id).unwrap().clone();

            ret.push((
                b"tree".to_vec(),
                collection_name.to_vec(),
                tree.iter().map(|kv_opt| {
                    let kv = kv_opt.unwrap();
                    vec![kv.0.to_vec(), kv.1.to_vec()]
                }),
            ));
        }

        ret
    }

    /// 从以前的数据库导入集合。
    ///
    /// # Panics
    ///
    /// 如果在尝试执行导入时发生任何 IO 问题，则会发生恐慌。
    ///
    /// # Examples
    ///
    /// 如果您想从一个版本的 melange_db 迁移到另一个版本，
    /// 您需要通过使用版本重命名引入两个版本：
    ///
    /// `Cargo.toml`:
    ///
    /// ```toml
    /// [dependencies]
    /// melange_db = "0.32"
    /// old_melange_db = { version = "0.31", package = "melange_db" }
    /// ```
    ///
    /// 在您的代码中，请记住旧版本的 melange_db 可能有不同的打开方式
    /// 比当前的 `melange_db::open` 方法：
    ///
    /// ```
    /// # use melange_db as old_melange_db;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let old = old_melange_db::open("my_old_db_import")?;
    ///
    /// // 可能是不同版本的 melange_db，
    /// // 导出类型是版本不可知的。
    /// let new = melange_db::open("my_new_db_import")?;
    ///
    /// let export = old.export();
    /// new.import(export);
    ///
    /// assert_eq!(old.checksum()?, new.checksum()?);
    /// # drop(old);
    /// # drop(new);
    /// # let _ = std::fs::remove_dir_all("my_old_db_import");
    /// # let _ = std::fs::remove_dir_all("my_new_db_import");
    /// # Ok(()) }
    /// ```
    pub fn import(
        &self,
        export: Vec<(
            CollectionType,
            CollectionName,
            impl Iterator<Item = Vec<Vec<u8>>>,
        )>,
    ) {
        for (collection_type, collection_name, collection_iter) in export {
            match collection_type {
                ref t if t == b"tree" => {
                    let tree = self
                        .open_tree(collection_name)
                        .expect("在导入期间无法打开新树");
                    for mut kv in collection_iter {
                        let v = kv
                            .pop()
                            .expect("无法从树导出中获取值");
                        let k = kv
                            .pop()
                            .expect("无法从树导出中获取键");
                        let old = tree.insert(k, v).expect(
                            "在树导入期间插入值失败",
                        );
                        assert!(
                            old.is_none(),
                            "导入正在覆盖现有数据"
                        );
                    }
                }
                other => panic!("未知集合类型 {:?}", other),
            }
        }
    }

    pub fn contains_tree<V: AsRef<[u8]>>(&self, name: V) -> io::Result<bool> {
        Ok(self.collection_name_mapping.get(name.as_ref())?.is_some())
    }

    pub fn drop_tree<V: AsRef<[u8]>>(&self, name: V) -> io::Result<bool> {
        let name_ref = name.as_ref();
        let trees = self.trees.lock();

        let tree = if let Some(collection_id_buf) =
            self.collection_name_mapping.get(name_ref)?
        {
            let collection_id = CollectionId(u64::from_le_bytes(
                collection_id_buf.as_ref().try_into().unwrap(),
            ));

            trees.get(&collection_id).unwrap()
        } else {
            return Ok(false);
        };

        tree.clear()?;

        self.collection_name_mapping.remove(name_ref)?;

        Ok(true)
    }

    /// 打开或创建一个新的磁盘支持的 [`Tree`]，具有自己的键空间，
    /// 可通过提供的标识符从 `Db` 访问。
    pub fn open_tree<V: AsRef<[u8]>>(
        &self,
        name: V,
    ) -> io::Result<Tree<LEAF_FANOUT>> {
        let name_ref = name.as_ref();
        let mut trees = self.trees.lock();

        if let Some(collection_id_buf) =
            self.collection_name_mapping.get(name_ref)?
        {
            let collection_id = CollectionId(u64::from_le_bytes(
                collection_id_buf.as_ref().try_into().unwrap(),
            ));

            let tree = trees.get(&collection_id).unwrap();

            return Ok(tree.clone());
        }

        let collection_id =
            CollectionId(self.collection_id_allocator.allocate());

        let initial_low_key = InlineArray::default();

        let empty_node = self.cache.allocate_default_node(collection_id);

        let index = Index::default();

        assert!(index.insert(initial_low_key, empty_node).is_none());

        let tree = Tree::new(
            collection_id,
            self.cache.clone(),
            index,
            self._shutdown_dropper.clone(),
        );

        self.collection_name_mapping
            .insert(name_ref, &collection_id.0.to_le_bytes())?;

        trees.insert(collection_id, tree.clone());

        Ok(tree)
    }
}

/// 智能flusher线程函数
fn smart_flusher<const LEAF_FANOUT: usize>(
    cache: ObjectCache<LEAF_FANOUT>,
    shutdown_signal: mpsc::Receiver<mpsc::Sender<()>>,
    config: SmartFlushConfig,
) {
    let mut scheduler = SmartFlushScheduler::new(config);
    let stats = scheduler.get_stats();

    let flush = || {
        let flush_res_res = std::panic::catch_unwind(|| cache.flush());
        match flush_res_res {
            Ok(Ok(_)) => {
                scheduler.notify_flush_completed();
                return;
            }
            Ok(Err(flush_failure)) => {
                error_log!(
                    "智能Db flusher 在刷新时遇到错误: {:?}",
                    flush_failure
                );
                cache.set_error(&flush_failure);
            }
            Err(panicked) => {
                error_log!(
                    "智能Db flusher 在刷新时发生恐慌: {:?}",
                    panicked
                );
                cache.set_error(&io::Error::other(
                    "智能Db flusher 在刷新时发生恐慌".to_string(),
                ));
            }
        }
        std::process::abort();
    };

    loop {
        let next_delay = scheduler.calculate_next_flush_delay();

        if let Ok(shutdown_sender) = shutdown_signal.recv_timeout(next_delay) {
            flush();

            cache.set_error(&io::Error::other(
                "系统已关闭".to_string(),
            ));

            assert!(cache.is_clean());

            drop(cache);

            if let Err(e) = shutdown_sender.send(()) {
                error_log!(
                    "智能Db flusher 无法向请求者确认关闭: {e:?}"
                );
            }
            debug_log!(
                "智能flush 线程在向请求者发出信号后终止"
            );
            return;
        }

        let before_flush = Instant::now();
        flush();
        let flush_duration = before_flush.elapsed();

        debug_log!("智能flush完成，耗时: {:?}", flush_duration);
    }
}

/// 这些类型提供允许整个系统被导出和导入以促进
/// 主要升级的信息。它完全由标准库类型组成，具有向前兼容性。
/// 注意这些定义的更改成本很高，因为它们会影响迁移路径。
type CollectionType = Vec<u8>;
type CollectionName = Vec<u8>;