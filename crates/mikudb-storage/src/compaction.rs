//! Compaction 模块
//!
//! 本模块定义 LSM-tree 的 compaction(压缩)配置和统计:
//! - **Compaction 配置**: LSM-tree 的多层级压缩参数
//! - **统计信息**: 压缩次数、字节数、耗时
//!
//! Compaction 用于:
//! - 合并 SSTable 文件,减少读放大
//! - 删除过期数据和已标记为删除的数据
//! - 均衡各层级的数据大小

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Compaction 配置
///
/// 定义 RocksDB 的 LSM-tree compaction 策略。
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// 是否启用自动 compaction
    pub enable_auto_compaction: bool,
    /// Compaction 间隔时间
    pub compaction_interval: Duration,
    /// 触发 compaction 的最小文件数
    pub min_files_to_compact: usize,
    /// 单次 compaction 的最大字节数(1GB)
    pub max_compaction_bytes: u64,
    /// 是否启用动态调整层级大小
    pub level_compaction_dynamic_level_bytes: bool,
    /// 目标 SSTable 文件大小(64MB)
    pub target_file_size_base: u64,
    /// 第 0 层的目标大小(256MB)
    pub max_bytes_for_level_base: u64,
    /// 每层大小倍增系数(10倍)
    pub max_bytes_for_level_multiplier: f64,
}

impl Default for CompactionConfig {
    /// # Brief
    /// 创建默认的 Compaction 配置
    ///
    /// 默认值:
    /// - 启用自动 compaction
    /// - 每 60 秒检查一次
    /// - 最少 4 个文件触发
    /// - 单次最大压缩 1GB
    /// - 动态调整层级大小
    /// - SSTable 目标大小 64MB
    /// - 第 0 层目标 256MB
    /// - 每层 10 倍倍增
    fn default() -> Self {
        Self {
            enable_auto_compaction: true,
            compaction_interval: Duration::from_secs(60),
            min_files_to_compact: 4,
            max_compaction_bytes: 1024 * 1024 * 1024,
            level_compaction_dynamic_level_bytes: true,
            target_file_size_base: 64 * 1024 * 1024,
            max_bytes_for_level_base: 256 * 1024 * 1024,
            max_bytes_for_level_multiplier: 10.0,
        }
    }
}

/// Compaction 统计信息
///
/// 记录 compaction 操作的执行情况。
#[derive(Debug, Default)]
pub struct CompactionStats {
    /// 已完成的 compaction 次数
    pub compactions_completed: AtomicU64,
    /// 已压缩的字节数
    pub bytes_compacted: AtomicU64,
    /// 总的 compaction 时间(毫秒)
    pub total_compaction_time_ms: AtomicU64,
    /// 待处理的 compaction 数量
    pub pending_compactions: AtomicU64,
}

impl CompactionStats {
    /// # Brief
    /// 记录一次 compaction 操作
    ///
    /// 更新完成次数、压缩字节数和总耗时。
    ///
    /// # Arguments
    /// * `bytes` - 本次压缩的字节数
    /// * `duration` - 本次压缩的耗时
    pub fn record_compaction(&self, bytes: u64, duration: Duration) {
        self.compactions_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_compacted.fetch_add(bytes, Ordering::Relaxed);
        self.total_compaction_time_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// # Brief
    /// 获取统计信息快照
    ///
    /// 返回当前的所有统计数据。
    ///
    /// # Returns
    /// 统计信息快照
    pub fn snapshot(&self) -> CompactionStatsSnapshot {
        CompactionStatsSnapshot {
            compactions_completed: self.compactions_completed.load(Ordering::Relaxed),
            bytes_compacted: self.bytes_compacted.load(Ordering::Relaxed),
            total_compaction_time_ms: self.total_compaction_time_ms.load(Ordering::Relaxed),
            pending_compactions: self.pending_compactions.load(Ordering::Relaxed),
        }
    }
}

/// Compaction 统计信息快照
///
/// 统计数据的不变快照,用于查询和展示。
#[derive(Debug, Clone)]
pub struct CompactionStatsSnapshot {
    /// 已完成的 compaction 次数
    pub compactions_completed: u64,
    /// 已压缩的字节数
    pub bytes_compacted: u64,
    /// 总的 compaction 时间(毫秒)
    pub total_compaction_time_ms: u64,
    /// 待处理的 compaction 数量
    pub pending_compactions: u64,
}

impl CompactionStatsSnapshot {
    /// # Brief
    /// 计算平均 compaction 时间
    ///
    /// # Returns
    /// 平均每次 compaction 的时间(毫秒)
    pub fn avg_compaction_time_ms(&self) -> f64 {
        if self.compactions_completed > 0 {
            // 总耗时 / 次数
            self.total_compaction_time_ms as f64 / self.compactions_completed as f64
        } else {
            0.0
        }
    }
}
