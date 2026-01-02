//! 数据库构建器模块
//!
//! 提供 Builder 模式创建和配置数据库实例。
//!
//! # 示例
//!
//! ```rust,ignore
//! use mikudb_core::DatabaseBuilder;
//!
//! let db = DatabaseBuilder::new("mydb")
//!     .data_dir("/var/lib/mikudb/data")
//!     .cache_size(2 * 1024 * 1024 * 1024)
//!     .enable_compression(true)
//!     .build()?;
//! ```

use crate::common::config::CompressionType;
use crate::common::MikuResult;
use crate::storage::StorageOptions;
use crate::Database;
use std::path::{Path, PathBuf};

pub struct DatabaseBuilder {
    name: String,
    data_dir: PathBuf,
    cache_size: usize,
    write_buffer_size: usize,
    max_write_buffer_number: i32,
    max_open_files: i32,
    compression: CompressionType,
    enable_statistics: bool,
    paranoid_checks: bool,
    for_openeuler: bool,

    #[cfg(target_os = "linux")]
    use_direct_reads: bool,
    #[cfg(target_os = "linux")]
    use_direct_writes: bool,
}

impl DatabaseBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        let defaults = StorageOptions::default();

        Self {
            name: name.into(),
            data_dir: defaults.data_dir,
            cache_size: defaults.cache_size,
            write_buffer_size: defaults.write_buffer_size,
            max_write_buffer_number: defaults.max_write_buffer_number,
            max_open_files: defaults.max_open_files,
            compression: defaults.compression,
            enable_statistics: defaults.enable_statistics,
            paranoid_checks: defaults.paranoid_checks,
            for_openeuler: false,

            #[cfg(target_os = "linux")]
            use_direct_reads: defaults.use_direct_reads,
            #[cfg(target_os = "linux")]
            use_direct_writes: defaults.use_direct_writes,
        }
    }

    pub fn data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.data_dir = path.as_ref().to_path_buf();
        self
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    pub fn cache_size_mb(mut self, mb: usize) -> Self {
        self.cache_size = mb * 1024 * 1024;
        self
    }

    pub fn cache_size_gb(mut self, gb: usize) -> Self {
        self.cache_size = gb * 1024 * 1024 * 1024;
        self
    }

    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.write_buffer_size = size;
        self
    }

    pub fn write_buffer_size_mb(mut self, mb: usize) -> Self {
        self.write_buffer_size = mb * 1024 * 1024;
        self
    }

    pub fn max_write_buffer_number(mut self, num: i32) -> Self {
        self.max_write_buffer_number = num;
        self
    }

    pub fn max_open_files(mut self, num: i32) -> Self {
        self.max_open_files = num;
        self
    }

    pub fn compression(mut self, compression: CompressionType) -> Self {
        self.compression = compression;
        self
    }

    pub fn enable_compression(mut self, enable: bool) -> Self {
        if enable {
            self.compression = CompressionType::Lz4;
        } else {
            self.compression = CompressionType::None;
        }
        self
    }

    pub fn use_lz4(mut self) -> Self {
        self.compression = CompressionType::Lz4;
        self
    }

    pub fn use_zstd(mut self) -> Self {
        self.compression = CompressionType::Zstd;
        self
    }

    pub fn no_compression(mut self) -> Self {
        self.compression = CompressionType::None;
        self
    }

    pub fn enable_statistics(mut self, enable: bool) -> Self {
        self.enable_statistics = enable;
        self
    }

    pub fn paranoid_checks(mut self, enable: bool) -> Self {
        self.paranoid_checks = enable;
        self
    }

    pub fn for_openeuler(mut self) -> Self {
        self.for_openeuler = true;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn use_direct_io(mut self, enable: bool) -> Self {
        self.use_direct_reads = enable;
        self.use_direct_writes = enable;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn use_direct_reads(mut self, enable: bool) -> Self {
        self.use_direct_reads = enable;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn use_direct_writes(mut self, enable: bool) -> Self {
        self.use_direct_writes = enable;
        self
    }

    pub fn build(self) -> MikuResult<Database> {
        let data_path = self.data_dir.join(&self.name);

        let options = if self.for_openeuler {
            let mut opts = StorageOptions::for_openeuler();
            opts.data_dir = data_path;
            opts.cache_size = self.cache_size;
            opts.write_buffer_size = self.write_buffer_size;
            opts.max_write_buffer_number = self.max_write_buffer_number;
            opts.max_open_files = self.max_open_files;
            opts.compression = self.compression;
            opts.enable_statistics = self.enable_statistics;
            opts.paranoid_checks = self.paranoid_checks;
            opts
        } else {
            StorageOptions {
                data_dir: data_path,
                cache_size: self.cache_size,
                write_buffer_size: self.write_buffer_size,
                max_write_buffer_number: self.max_write_buffer_number,
                max_open_files: self.max_open_files,
                compression: self.compression,
                enable_statistics: self.enable_statistics,
                paranoid_checks: self.paranoid_checks,
                enable_wal: true,
                wal_sync_on_write: false,

                #[cfg(target_os = "linux")]
                use_direct_reads: self.use_direct_reads,
                #[cfg(target_os = "linux")]
                use_direct_writes: self.use_direct_writes,
                #[cfg(target_os = "linux")]
                allow_mmap_reads: false,
                #[cfg(target_os = "linux")]
                allow_mmap_writes: false,
            }
        };

        Database::open_with_options(self.name, options)
    }
}

pub struct StorageOptionsBuilder {
    options: StorageOptions,
}

impl StorageOptionsBuilder {
    pub fn new() -> Self {
        Self {
            options: StorageOptions::default(),
        }
    }

    pub fn for_openeuler() -> Self {
        Self {
            options: StorageOptions::for_openeuler(),
        }
    }

    pub fn data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.options.data_dir = path.as_ref().to_path_buf();
        self
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.options.cache_size = size;
        self
    }

    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.options.write_buffer_size = size;
        self
    }

    pub fn max_write_buffer_number(mut self, num: i32) -> Self {
        self.options.max_write_buffer_number = num;
        self
    }

    pub fn max_open_files(mut self, num: i32) -> Self {
        self.options.max_open_files = num;
        self
    }

    pub fn compression(mut self, compression: CompressionType) -> Self {
        self.options.compression = compression;
        self
    }

    pub fn enable_statistics(mut self, enable: bool) -> Self {
        self.options.enable_statistics = enable;
        self
    }

    pub fn paranoid_checks(mut self, enable: bool) -> Self {
        self.options.paranoid_checks = enable;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn use_direct_io(mut self, enable: bool) -> Self {
        self.options.use_direct_reads = enable;
        self.options.use_direct_writes = enable;
        self
    }

    pub fn build(self) -> StorageOptions {
        self.options
    }
}

impl Default for StorageOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_builder_basic() {
        let dir = tempdir().unwrap();

        let db = DatabaseBuilder::new("test")
            .data_dir(dir.path())
            .build()
            .unwrap();

        assert_eq!(db.name(), "test");
    }

    #[test]
    fn test_database_builder_with_cache() {
        let dir = tempdir().unwrap();

        let db = DatabaseBuilder::new("test")
            .data_dir(dir.path())
            .cache_size_mb(512)
            .build()
            .unwrap();

        assert_eq!(db.name(), "test");
    }

    #[test]
    fn test_database_builder_with_compression() {
        let dir = tempdir().unwrap();

        let db = DatabaseBuilder::new("test")
            .data_dir(dir.path())
            .use_zstd()
            .build()
            .unwrap();

        assert_eq!(db.name(), "test");
    }

    #[test]
    fn test_storage_options_builder() {
        let dir = tempdir().unwrap();

        let options = StorageOptionsBuilder::new()
            .data_dir(dir.path())
            .cache_size(256 * 1024 * 1024)
            .compression(CompressionType::Lz4)
            .build();

        assert_eq!(options.cache_size, 256 * 1024 * 1024);
    }
}
