//! 存储引擎模块
//!
//! 基于 RocksDB 的高性能存储引擎实现。
//!
//! # OpenEuler 适配亮点
//!
//! - 支持 Direct I/O 优化，减少内存拷贝
//! - 自动检测鲲鹏 CPU 并优化缓冲区大小
//! - 支持 NUMA 感知的内存分配
//! - 针对 ARM64 架构优化的块大小配置

use crate::{StorageError, StorageResult};
use mikudb_boml::{codec, Document, BomlValue};
use mikudb_common::config::CompressionType;
use mikudb_common::platform::{linux, Platform};
use mikudb_common::{CollectionName, DatabaseName, DocumentId, ObjectId};
use parking_lot::RwLock;
use rocksdb::{
    BlockBasedOptions, Cache, ColumnFamily, ColumnFamilyDescriptor, DBCompactionStyle,
    DBCompressionType, Env, Options, ReadOptions, WriteOptions, DB,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

const METADATA_CF: &str = "_metadata";
const SYSTEM_CF: &str = "_system";
const DEFAULT_CF: &str = "default";

/// 存储引擎配置选项
///
/// 用于配置 RocksDB 底层存储的各种参数
#[derive(Debug, Clone)]
pub struct StorageOptions {
    pub data_dir: PathBuf,
    pub cache_size: usize,
    pub write_buffer_size: usize,
    pub max_write_buffer_number: i32,
    pub max_open_files: i32,
    pub compression: CompressionType,
    pub enable_statistics: bool,
    pub paranoid_checks: bool,

    #[cfg(target_os = "linux")]
    pub use_direct_reads: bool,
    #[cfg(target_os = "linux")]
    pub use_direct_writes: bool,
    #[cfg(target_os = "linux")]
    pub allow_mmap_reads: bool,
    #[cfg(target_os = "linux")]
    pub allow_mmap_writes: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        let platform = Platform::current();

        #[cfg(target_os = "linux")]
        let (use_direct_reads, use_direct_writes) = if platform.is_openeuler() {
            let config = linux::openeuler::get_recommended_config();
            (config.use_direct_io, config.use_direct_io)
        } else {
            (true, true)
        };

        Self {
            data_dir: PathBuf::from("/var/lib/mikudb/data"),
            cache_size: 1024 * 1024 * 1024,
            write_buffer_size: 64 * 1024 * 1024,
            max_write_buffer_number: 4,
            max_open_files: 10000,
            compression: CompressionType::Lz4,
            enable_statistics: true,
            paranoid_checks: true,

            #[cfg(target_os = "linux")]
            use_direct_reads,
            #[cfg(target_os = "linux")]
            use_direct_writes,
            #[cfg(target_os = "linux")]
            allow_mmap_reads: false,
            #[cfg(target_os = "linux")]
            allow_mmap_writes: false,
        }
    }
}

impl StorageOptions {
    /// 创建 OpenEuler 优化的配置
    ///
    /// # Brief
    /// 根据 OpenEuler 平台特性自动配置最优参数，包括鲲鹏 CPU 优化
    ///
    /// # OpenEuler 适配亮点
    /// - 自动检测鲲鹏 CPU 并调整写缓冲区大小
    /// - 根据系统内存调整缓存大小
    /// - 启用 Direct I/O 效果更佳
    ///
    /// # Returns
    /// OpenEuler 优化的 StorageOptions
    pub fn for_openeuler() -> Self {
        let mut opts = Self::default();

        #[cfg(target_os = "linux")]
        {
            let config = linux::openeuler::get_recommended_config();

            opts.cache_size = config.recommended_cache_size as usize;
            opts.write_buffer_size = config.recommended_write_buffer;

            if config.is_kunpeng {
                opts.max_write_buffer_number = 6;
                opts.write_buffer_size = 128 * 1024 * 1024;
            }

            opts.use_direct_reads = config.use_direct_io;
            opts.use_direct_writes = config.use_direct_io;
        }

        opts
    }
}

/// 存储引擎
///
/// 基于 RocksDB 的文档存储引擎，提供集合管理和文档 CRUD 操作
pub struct StorageEngine {
    db: Arc<DB>,
    options: StorageOptions,
    collections: RwLock<HashMap<String, Arc<crate::collection::Collection>>>,
    block_cache: Arc<Cache>,
}

impl StorageEngine {
    /// 打开存储引擎
    ///
    /// # Brief
    /// 使用指定配置打开或创建 RocksDB 数据库
    ///
    /// # OpenEuler 适配亮点
    /// - 自动检测平台并应用优化配置
    /// - 记录 NUMA 节点、io_uring、大页内存支持情况
    ///
    /// # Arguments
    /// * `options` - 存储引擎配置
    ///
    /// # Returns
    /// 成功返回 StorageEngine，失败返回错误
    pub fn open(options: StorageOptions) -> StorageResult<Self> {
        std::fs::create_dir_all(&options.data_dir)?;

        let platform = Platform::current();
        if platform.is_openeuler() {
            info!("OpenEuler detected - applying optimized settings");
            #[cfg(target_os = "linux")]
            {
                let config = linux::openeuler::get_recommended_config();
                if config.is_kunpeng {
                    info!("Kunpeng CPU detected - enabling ARM64 optimizations");
                }
                info!(
                    "NUMA nodes: {}, io_uring: {}, huge_pages: {}",
                    config.numa_nodes, config.use_io_uring, config.use_huge_pages
                );
            }
        }

        let block_cache = Cache::new_lru_cache(options.cache_size);
        let mut block_opts = BlockBasedOptions::default();
        block_opts.set_block_cache(&block_cache);
        block_opts.set_block_size(16 * 1024);
        block_opts.set_cache_index_and_filter_blocks(true);
        block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
        block_opts.set_bloom_filter(10.0, false);

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        db_opts.set_max_open_files(options.max_open_files);
        db_opts.set_write_buffer_size(options.write_buffer_size);
        db_opts.set_max_write_buffer_number(options.max_write_buffer_number);
        db_opts.set_min_write_buffer_number_to_merge(2);

        let compression = match options.compression {
            CompressionType::None => DBCompressionType::None,
            CompressionType::Lz4 => DBCompressionType::Lz4,
            CompressionType::Zstd => DBCompressionType::Zstd,
        };
        db_opts.set_compression_type(compression);

        db_opts.set_compaction_style(DBCompactionStyle::Level);
        db_opts.set_level_compaction_dynamic_level_bytes(true);
        db_opts.set_max_background_jobs(4);
        db_opts.set_bytes_per_sync(1024 * 1024);
        db_opts.set_wal_bytes_per_sync(1024 * 1024);

        if options.enable_statistics {
            db_opts.enable_statistics();
        }

        if options.paranoid_checks {
            db_opts.set_paranoid_checks(true);
        }

        #[cfg(target_os = "linux")]
        {
            if options.use_direct_reads {
                db_opts.set_use_direct_reads(true);
            }
            if options.use_direct_writes {
                db_opts.set_use_direct_io_for_flush_and_compaction(true);
            }
            db_opts.set_allow_mmap_reads(options.allow_mmap_reads);
            db_opts.set_allow_mmap_writes(options.allow_mmap_writes);

            if platform.is_openeuler() {
                let numa_nodes = linux::get_numa_node_count();
                if numa_nodes > 1 {
                    info!("Multi-NUMA system detected ({} nodes)", numa_nodes);
                }
            }
        }

        db_opts.set_block_based_table_factory(&block_opts);

        let cf_names = Self::get_existing_cf_names(&options.data_dir)?;
        let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(compression);
                ColumnFamilyDescriptor::new(name, cf_opts)
            })
            .collect();

        let db = if cf_descriptors.is_empty() {
            let mut cf_opts = Options::default();
            cf_opts.set_compression_type(compression);

            DB::open_cf_descriptors(
                &db_opts,
                &options.data_dir,
                vec![
                    ColumnFamilyDescriptor::new(DEFAULT_CF, cf_opts.clone()),
                    ColumnFamilyDescriptor::new(METADATA_CF, cf_opts.clone()),
                    ColumnFamilyDescriptor::new(SYSTEM_CF, cf_opts),
                ],
            )?
        } else {
            DB::open_cf_descriptors(&db_opts, &options.data_dir, cf_descriptors)?
        };

        info!("Storage engine opened at {:?}", options.data_dir);

        Ok(Self {
            db: Arc::new(db),
            options,
            collections: RwLock::new(HashMap::new()),
            block_cache: Arc::new(block_cache),
        })
    }

    fn get_existing_cf_names(path: &Path) -> StorageResult<Vec<String>> {
        if !path.exists() {
            return Ok(vec![
                DEFAULT_CF.to_string(),
                METADATA_CF.to_string(),
                SYSTEM_CF.to_string(),
            ]);
        }

        match DB::list_cf(&Options::default(), path) {
            Ok(names) => {
                let mut result = names;
                if !result.contains(&METADATA_CF.to_string()) {
                    result.push(METADATA_CF.to_string());
                }
                if !result.contains(&SYSTEM_CF.to_string()) {
                    result.push(SYSTEM_CF.to_string());
                }
                Ok(result)
            }
            Err(_) => Ok(vec![
                DEFAULT_CF.to_string(),
                METADATA_CF.to_string(),
                SYSTEM_CF.to_string(),
            ]),
        }
    }

    /// 创建集合
    ///
    /// # Brief
    /// 创建新的文档集合（Column Family）
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 成功返回集合的 Arc 引用，如果集合已存在则返回错误
    pub fn create_collection(&self, name: &str) -> StorageResult<Arc<crate::collection::Collection>> {
        let mut collections = self.collections.write();

        if collections.contains_key(name) {
            return Err(StorageError::CollectionExists(name.to_string()));
        }

        let cf_opts = Options::default();
        self.db.create_cf(name, &cf_opts)?;

        let collection = Arc::new(crate::collection::Collection::new(
            name.to_string(),
            self.db.clone(),
        ));

        collections.insert(name.to_string(), collection.clone());

        let metadata_cf = self.db.cf_handle(METADATA_CF).ok_or_else(|| {
            StorageError::Internal("Metadata CF not found".to_string())
        })?;
        let key = format!("collection:{}", name);
        let metadata = serde_json::json!({
            "name": name,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        self.db.put_cf(
            &metadata_cf,
            key.as_bytes(),
            serde_json::to_vec(&metadata).unwrap(),
        )?;

        info!("Created collection: {}", name);
        Ok(collection)
    }

    /// 获取集合
    ///
    /// # Brief
    /// 获取已存在的集合
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 成功返回集合的 Arc 引用，如果集合不存在则返回错误
    pub fn get_collection(&self, name: &str) -> StorageResult<Arc<crate::collection::Collection>> {
        let collections = self.collections.read();

        if let Some(collection) = collections.get(name) {
            return Ok(collection.clone());
        }
        drop(collections);

        if self.db.cf_handle(name).is_some() {
            let mut collections = self.collections.write();
            let collection = Arc::new(crate::collection::Collection::new(
                name.to_string(),
                self.db.clone(),
            ));
            collections.insert(name.to_string(), collection.clone());
            return Ok(collection);
        }

        Err(StorageError::CollectionNotFound(name.to_string()))
    }

    /// 获取或创建集合
    ///
    /// # Brief
    /// 获取指定集合，如果不存在则自动创建
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 返回集合的 Arc 引用
    pub fn get_or_create_collection(&self, name: &str) -> StorageResult<Arc<crate::collection::Collection>> {
        match self.get_collection(name) {
            Ok(collection) => Ok(collection),
            Err(StorageError::CollectionNotFound(_)) => self.create_collection(name),
            Err(e) => Err(e),
        }
    }

    /// 删除集合
    ///
    /// # Brief
    /// 删除指定的集合及其所有数据
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 成功返回 Ok(())，失败返回错误
    pub fn drop_collection(&self, name: &str) -> StorageResult<()> {
        let mut collections = self.collections.write();

        collections.remove(name);

        self.db.drop_cf(name)?;

        let metadata_cf = self.db.cf_handle(METADATA_CF).ok_or_else(|| {
            StorageError::Internal("Metadata CF not found".to_string())
        })?;
        let key = format!("collection:{}", name);
        self.db.delete_cf(&metadata_cf, key.as_bytes())?;

        info!("Dropped collection: {}", name);
        Ok(())
    }

    /// 列出所有集合
    ///
    /// # Brief
    /// 返回数据库中所有集合的名称列表
    ///
    /// # Returns
    /// 集合名称的向量
    pub fn list_collections(&self) -> StorageResult<Vec<String>> {
        let metadata_cf = self.db.cf_handle(METADATA_CF).ok_or_else(|| {
            StorageError::Internal("Metadata CF not found".to_string())
        })?;

        let prefix = b"collection:";
        let mut collections = Vec::new();

        let iter = self.db.prefix_iterator_cf(&metadata_cf, prefix);
        for item in iter {
            let (key, _) = item?;
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Some(name) = key_str.strip_prefix("collection:") {
                    collections.push(name.to_string());
                }
            }
        }

        Ok(collections)
    }

    /// 压缩数据库
    ///
    /// # Brief
    /// 触发 RocksDB 的全库压缩操作以回收空间
    ///
    /// # Returns
    /// 成功返回 Ok(())
    pub fn compact(&self) -> StorageResult<()> {
        info!("Starting compaction");
        self.db.compact_range::<&[u8], &[u8]>(None, None);
        info!("Compaction completed");
        Ok(())
    }

    /// 刷新数据到磁盘
    ///
    /// # Brief
    /// 将内存中的数据刷新到磁盘，确保数据持久化
    ///
    /// # Returns
    /// 成功返回 Ok(())
    pub fn flush(&self) -> StorageResult<()> {
        self.db.flush()?;
        Ok(())
    }

    /// 获取 RocksDB 统计信息
    ///
    /// # Brief
    /// 返回 RocksDB 的详细统计信息
    ///
    /// # Returns
    /// 统计信息字符串，如果未启用则返回 None
    pub fn get_statistics(&self) -> Option<String> {
        self.db.property_value("rocksdb.stats").ok().flatten()
    }

    /// 获取数据库约估大小
    ///
    /// # Brief
    /// 返回数据库数据的约估大小（字节）
    ///
    /// # Returns
    /// 约估的字节数
    pub fn get_approximate_size(&self) -> u64 {
        self.db
            .property_int_value("rocksdb.estimate-live-data-size")
            .unwrap_or(Some(0))
            .unwrap_or(0)
    }

    /// 获取数据库路径
    ///
    /// # Brief
    /// 返回数据库文件的存储路径
    ///
    /// # Returns
    /// 数据库路径的引用
    pub fn path(&self) -> &Path {
        self.db.path()
    }
}

impl Drop for StorageEngine {
    fn drop(&mut self) {
        info!("Closing storage engine");
        if let Err(e) = self.flush() {
            warn!("Error flushing on close: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_storage() {
        let dir = tempdir().unwrap();
        let options = StorageOptions {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };

        let engine = StorageEngine::open(options).unwrap();
        assert!(engine.list_collections().unwrap().is_empty());
    }

    #[test]
    fn test_create_collection() {
        let dir = tempdir().unwrap();
        let options = StorageOptions {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };

        let engine = StorageEngine::open(options).unwrap();
        engine.create_collection("test").unwrap();

        let collections = engine.list_collections().unwrap();
        assert!(collections.contains(&"test".to_string()));
    }
}
