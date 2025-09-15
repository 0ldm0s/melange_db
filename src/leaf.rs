use crate::*;

/// 增量序列化变更跟踪结构
/// 用于跟踪leaf节点自上次完整序列化以来的变更
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IncrementalChanges {
    /// 修改的键值对
    pub modified_keys: Vec<InlineArray>,
    /// 修改的值
    pub modified_values: Vec<InlineArray>,
    /// 删除的键
    pub deleted_keys: Vec<InlineArray>,
    /// 基础序列化版本号
    pub base_version: u64,
    /// 当前版本号
    pub current_version: u64,
}

impl IncrementalChanges {
    pub fn new(base_version: u64) -> Self {
        Self {
            modified_keys: Vec::new(),
            modified_values: Vec::new(),
            deleted_keys: Vec::new(),
            base_version,
            current_version: base_version,
        }
    }

    pub fn add_insert(&mut self, key: InlineArray, value: InlineArray) {
        // 如果键在删除列表中，先移除
        if let Some(pos) = self.deleted_keys.iter().position(|k| k == &key) {
            self.deleted_keys.remove(pos);
        }

        // 如果键已在修改列表中，更新值
        if let Some(pos) = self.modified_keys.iter().position(|k| k == &key) {
            self.modified_values[pos] = value;
        } else {
            self.modified_keys.push(key);
            self.modified_values.push(value);
        }

        self.current_version += 1;
    }

    pub fn add_delete(&mut self, key: InlineArray) {
        // 如果键在修改列表中，移除
        if let Some(pos) = self.modified_keys.iter().position(|k| k == &key) {
            self.modified_keys.remove(pos);
            self.modified_values.remove(pos);
        }

        // 如果键不在删除列表中，添加
        if !self.deleted_keys.contains(&key) {
            self.deleted_keys.push(key);
        }

        self.current_version += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.modified_keys.is_empty() && self.deleted_keys.is_empty()
    }

    pub fn clear(&mut self) {
        self.modified_keys.clear();
        self.modified_values.clear();
        self.deleted_keys.clear();
        self.base_version = self.current_version;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Leaf<const LEAF_FANOUT: usize> {
    pub lo: InlineArray,
    pub hi: Option<InlineArray>,
    pub prefix_length: usize,
    data: stack_map::StackMap<InlineArray, InlineArray, LEAF_FANOUT>,
    pub in_memory_size: usize,
    pub mutation_count: u64,
    #[serde(skip)]
    pub dirty_flush_epoch: Option<FlushEpoch>,
    #[serde(skip)]
    pub page_out_on_flush: Option<FlushEpoch>,
    #[serde(skip)]
    pub deleted: Option<FlushEpoch>,
    #[serde(skip)]
    pub max_unflushed_epoch: Option<FlushEpoch>,
    /// 增量变更跟踪
    #[serde(skip)]
    pub incremental_changes: Option<IncrementalChanges>,
    /// 上次完整序列化的版本
    #[serde(skip)]
    pub last_serialized_version: u64,
    /// 是否启用增量序列化
    #[serde(skip)]
    pub incremental_serialization_enabled: bool,
}

impl<const LEAF_FANOUT: usize> Leaf<LEAF_FANOUT> {
    pub(crate) fn empty() -> Leaf<LEAF_FANOUT> {
        Leaf {
            lo: InlineArray::default(),
            hi: None,
            prefix_length: 0,
            data: stack_map::StackMap::default(),
            // this does not need to be marked as dirty until it actually
            // receives inserted data
            dirty_flush_epoch: None,
            in_memory_size: std::mem::size_of::<Leaf<LEAF_FANOUT>>(),
            mutation_count: 0,
            page_out_on_flush: None,
            deleted: None,
            max_unflushed_epoch: None,
            incremental_changes: None,
            last_serialized_version: 0,
            incremental_serialization_enabled: false,
        }
    }

    /// 启用增量序列化
    pub(crate) fn enable_incremental_serialization(&mut self) {
        self.incremental_serialization_enabled = true;
        self.incremental_changes = Some(IncrementalChanges::new(self.last_serialized_version));
    }

    /// 禁用增量序列化
    pub(crate) fn disable_incremental_serialization(&mut self) {
        self.incremental_serialization_enabled = false;
        self.incremental_changes = None;
    }

    /// 检查是否应该使用增量序列化
    fn should_use_incremental_serialization(&self) -> bool {
        self.incremental_serialization_enabled
            && self.incremental_changes.as_ref().map_or(false, |changes| {
                !changes.is_empty() && changes.modified_keys.len() < self.data.len() / 2
            })
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(crate) fn set_dirty_epoch(&mut self, epoch: FlushEpoch) {
        assert!(self.deleted.is_none());
        if let Some(current_epoch) = self.dirty_flush_epoch {
            assert!(current_epoch <= epoch);
        }
        if self.page_out_on_flush < Some(epoch) {
            self.page_out_on_flush = None;
        }
        self.dirty_flush_epoch = Some(epoch);
    }

    fn prefix(&self) -> &[u8] {
        assert!(self.deleted.is_none());
        &self.lo[..self.prefix_length]
    }

    pub(crate) fn get(&self, key: &[u8]) -> Option<&InlineArray> {
        assert!(self.deleted.is_none());
        assert!(key.starts_with(self.prefix()));
        let prefixed_key = &key[self.prefix_length..];
        self.data.get(prefixed_key)
    }

    pub(crate) fn insert(
        &mut self,
        key: InlineArray,
        value: InlineArray,
    ) -> Option<InlineArray> {
        assert!(self.deleted.is_none());
        assert!(key.starts_with(self.prefix()));
        let prefixed_key = key[self.prefix_length..].into();

        let old_value = self.data.insert(prefixed_key, value.clone());

        // 跟踪增量变更
        if self.incremental_serialization_enabled {
            if let Some(changes) = &mut self.incremental_changes {
                changes.add_insert(key, value);
            }
        }

        old_value
    }

    pub(crate) fn remove(&mut self, key: &[u8]) -> Option<InlineArray> {
        assert!(self.deleted.is_none());
        let prefix = self.prefix();
        assert!(key.starts_with(prefix));
        let partial_key = &key[self.prefix_length..];

        let old_value = self.data.remove(partial_key);

        // 跟踪增量变更
        if self.incremental_serialization_enabled {
            if let Some(changes) = &mut self.incremental_changes {
                changes.add_delete(key.into());
            }
        }

        old_value
    }

    pub(crate) fn merge_from(&mut self, other: &mut Self) {
        assert!(self.is_empty());

        self.hi = other.hi.clone();

        let new_prefix_len = if let Some(hi) = &self.hi {
            self.lo.iter().zip(hi.iter()).take_while(|(l, r)| l == r).count()
        } else {
            0
        };

        assert_eq!(self.lo[..new_prefix_len], other.lo[..new_prefix_len]);

        // self.prefix_length is not read because it's expected to be
        // initialized here.
        self.prefix_length = new_prefix_len;

        if self.prefix() == other.prefix() {
            self.data = std::mem::take(&mut other.data);
            return;
        }

        assert!(
            self.prefix_length < other.prefix_length,
            "self: {:?} other: {:?}",
            self,
            other
        );

        let unshifted_key_amount = other.prefix_length - self.prefix_length;
        let unshifted_prefix = &other.lo
            [other.prefix_length - unshifted_key_amount..other.prefix_length];

        for (k, v) in other.data.iter() {
            let mut unshifted_key =
                Vec::with_capacity(unshifted_prefix.len() + k.len());
            unshifted_key.extend_from_slice(unshifted_prefix);
            unshifted_key.extend_from_slice(k);
            self.data.insert(unshifted_key.into(), v.clone());
        }

        assert_eq!(other.data.len(), self.data.len());

        #[cfg(feature = "for-internal-testing-only")]
        assert_eq!(
            self.iter().collect::<Vec<_>>(),
            other.iter().collect::<Vec<_>>(),
            "self: {:#?} \n other: {:#?}\n",
            self,
            other
        );
    }

    pub(crate) fn iter(
        &self,
    ) -> impl Iterator<Item = (InlineArray, InlineArray)> {
        let prefix = self.prefix();
        self.data.iter().map(|(k, v)| {
            let mut unshifted_key = Vec::with_capacity(prefix.len() + k.len());
            unshifted_key.extend_from_slice(prefix);
            unshifted_key.extend_from_slice(k);
            (unshifted_key.into(), v.clone())
        })
    }

    /// 序列化leaf节点，支持增量序列化
    pub(crate) fn serialize(&self, zstd_compression_level: i32) -> Vec<u8> {
        if self.should_use_incremental_serialization() {
            self.serialize_incremental(zstd_compression_level)
        } else {
            self.serialize_full(zstd_compression_level)
        }
    }

    /// 完整序列化
    fn serialize_full(&self, zstd_compression_level: i32) -> Vec<u8> {
        let mut ret = vec![];

        let mut zstd_enc =
            zstd::stream::Encoder::new(&mut ret, zstd_compression_level)
                .unwrap();

        bincode::serialize_into(&mut zstd_enc, self).unwrap();

        zstd_enc.finish().unwrap();

        ret
    }

    /// 增量序列化
    fn serialize_incremental(&self, zstd_compression_level: i32) -> Vec<u8> {
        let changes = self.incremental_changes.as_ref().unwrap();

        let mut ret = vec![];

        // 写入增量序列化标记
        ret.push(0xFF); // 增量序列化标记

        let mut zstd_enc =
            zstd::stream::Encoder::new(&mut ret, zstd_compression_level)
                .unwrap();

        // 序列化增量变更
        bincode::serialize_into(&mut zstd_enc, changes).unwrap();

        zstd_enc.finish().unwrap();

        log::debug!("增量序列化: {} 个变更，大小: {} 字节",
                   changes.modified_keys.len() + changes.deleted_keys.len(),
                   ret.len());

        ret
    }

    /// 反序列化leaf节点，自动检测增量序列化
    pub(crate) fn deserialize(
        buf: &[u8],
    ) -> std::io::Result<Box<Leaf<LEAF_FANOUT>>> {
        if buf.len() > 0 && buf[0] == 0xFF {
            // 增量序列化数据
            Self::deserialize_incremental(buf)
        } else {
            // 完整序列化数据
            Self::deserialize_full(buf)
        }
    }

    /// 反序列化完整数据
    fn deserialize_full(buf: &[u8]) -> std::io::Result<Box<Leaf<LEAF_FANOUT>>> {
        let zstd_decoded = zstd::stream::decode_all(buf).unwrap();
        let mut leaf: Box<Leaf<LEAF_FANOUT>> =
            bincode::deserialize(&zstd_decoded).unwrap();

        // 使用解压后的缓冲区长度作为内存大小的粗略估计
        leaf.in_memory_size = zstd_decoded.len();

        Ok(leaf)
    }

    /// 反序列化增量数据
    fn deserialize_incremental(buf: &[u8]) -> std::io::Result<Box<Leaf<LEAF_FANOUT>>> {
        let zstd_decoded = zstd::stream::decode_all(&buf[1..]).unwrap();
        let changes: IncrementalChanges = bincode::deserialize(&zstd_decoded).unwrap();
        let base_version = changes.base_version;
        let current_version = changes.current_version;

        // 注意：增量反序列化需要与基础数据合并
        // 这里返回一个空的leaf，实际应用中需要先加载基础数据
        let mut leaf = Box::new(Leaf::empty());
        leaf.incremental_changes = Some(changes);
        leaf.last_serialized_version = base_version;

        log::debug!("增量反序列化: 基础版本 {}, 当前版本 {}",
                   base_version, current_version);

        Ok(leaf)
    }

    /// 应用增量变更到当前leaf
    pub(crate) fn apply_incremental_changes(&mut self, changes: &IncrementalChanges) {
        // 应用修改
        for (key, value) in changes.modified_keys.iter().zip(changes.modified_values.iter()) {
            let prefixed_key = &key[self.prefix_length..];
            self.data.insert(prefixed_key.into(), value.clone());
        }

        // 应用删除
        for key in &changes.deleted_keys {
            let prefixed_key = &key[self.prefix_length..];
            self.data.remove(prefixed_key);
        }

        self.last_serialized_version = changes.current_version;
        self.set_in_memory_size();
    }

    /// 重置增量变更跟踪
    pub(crate) fn reset_incremental_changes(&mut self) {
        if let Some(changes) = &mut self.incremental_changes {
            changes.clear();
            self.last_serialized_version = changes.current_version;
        }
    }

    fn set_in_memory_size(&mut self) {
        self.in_memory_size = std::mem::size_of::<Leaf<LEAF_FANOUT>>()
            + self.hi.as_ref().map(|h| h.len()).unwrap_or(0)
            + self.lo.len()
            + self.data.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>();
    }

    pub(crate) fn split_if_full(
        &mut self,
        new_epoch: FlushEpoch,
        allocator: &ObjectCache<LEAF_FANOUT>,
        collection_id: CollectionId,
    ) -> Option<(InlineArray, Object<LEAF_FANOUT>)> {
        if self.data.is_full() {
            let original_len = self.data.len();

            let old_prefix_len = self.prefix_length;
            // split
            let split_offset = if self.lo.is_empty() {
                // split left-most shard almost at the beginning for
                // optimizing downward-growing workloads
                1
            } else if self.hi.is_none() {
                // split right-most shard almost at the end for
                // optimizing upward-growing workloads
                self.data.len() - 2
            } else {
                self.data.len() / 2
            };

            let data = self.data.split_off(split_offset);

            let left_max = &self.data.last().unwrap().0;
            let right_min = &data.first().unwrap().0;

            // suffix truncation attempts to shrink the split key
            // so that shorter keys bubble up into the index
            let splitpoint_length = right_min
                .iter()
                .zip(left_max.iter())
                .take_while(|(a, b)| a == b)
                .count()
                + 1;

            let mut split_vec =
                Vec::with_capacity(self.prefix_length + splitpoint_length);
            split_vec.extend_from_slice(self.prefix());
            split_vec.extend_from_slice(&right_min[..splitpoint_length]);
            let split_key = InlineArray::from(split_vec);

            let rhs_id = allocator.allocate_object_id(new_epoch);

            log::trace!(
                "split leaf {:?} at split key: {:?} into new {:?} at {:?}",
                self.lo,
                split_key,
                rhs_id,
                new_epoch,
            );

            let mut rhs = Leaf {
                dirty_flush_epoch: Some(new_epoch),
                hi: self.hi.clone(),
                lo: split_key.clone(),
                prefix_length: 0,
                in_memory_size: 0,
                data,
                mutation_count: 0,
                page_out_on_flush: None,
                deleted: None,
                max_unflushed_epoch: None,
                incremental_changes: None,
                last_serialized_version: 0,
                incremental_serialization_enabled: self.incremental_serialization_enabled,
            };

            // 如果启用增量序列化，为新leaf也启用
            if self.incremental_serialization_enabled {
                rhs.enable_incremental_serialization();
            }

            rhs.shorten_keys_after_split(old_prefix_len);

            rhs.set_in_memory_size();

            self.hi = Some(split_key.clone());

            self.shorten_keys_after_split(old_prefix_len);

            self.set_in_memory_size();

            assert_eq!(self.hi.as_ref().unwrap(), &split_key);
            assert_eq!(rhs.lo, &split_key);
            assert_eq!(rhs.data.len() + self.data.len(), original_len);

            let rhs_node = Object {
                object_id: rhs_id,
                collection_id,
                low_key: split_key.clone(),
                inner: Arc::new(RwLock::new(CacheBox {
                    leaf: Some(Box::new(rhs)),
                    logged_index: BTreeMap::default(),
                })),
            };

            return Some((split_key, rhs_node));
        }

        None
    }

    pub(crate) fn shorten_keys_after_split(&mut self, old_prefix_len: usize) {
        let Some(hi) = self.hi.as_ref() else { return };

        let new_prefix_len =
            self.lo.iter().zip(hi.iter()).take_while(|(l, r)| l == r).count();

        assert_eq!(self.lo[..new_prefix_len], hi[..new_prefix_len]);

        // self.prefix_length is not read because it's expected to be
        // initialized here.
        self.prefix_length = new_prefix_len;

        if new_prefix_len == old_prefix_len {
            return;
        }

        assert!(
            new_prefix_len > old_prefix_len,
            "expected new prefix length of {} to be greater than the pre-split prefix length of {} for node {:?}",
            new_prefix_len,
            old_prefix_len,
            self
        );

        let key_shift = new_prefix_len - old_prefix_len;

        for (k, v) in std::mem::take(&mut self.data).iter() {
            self.data.insert(k[key_shift..].into(), v.clone());
        }
    }
}