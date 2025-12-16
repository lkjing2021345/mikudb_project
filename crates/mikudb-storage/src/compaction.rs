use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub enable_auto_compaction: bool,
    pub compaction_interval: Duration,
    pub min_files_to_compact: usize,
    pub max_compaction_bytes: u64,
    pub level_compaction_dynamic_level_bytes: bool,
    pub target_file_size_base: u64,
    pub max_bytes_for_level_base: u64,
    pub max_bytes_for_level_multiplier: f64,
}

impl Default for CompactionConfig {
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

#[derive(Debug, Default)]
pub struct CompactionStats {
    pub compactions_completed: AtomicU64,
    pub bytes_compacted: AtomicU64,
    pub total_compaction_time_ms: AtomicU64,
    pub pending_compactions: AtomicU64,
}

impl CompactionStats {
    pub fn record_compaction(&self, bytes: u64, duration: Duration) {
        self.compactions_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_compacted.fetch_add(bytes, Ordering::Relaxed);
        self.total_compaction_time_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CompactionStatsSnapshot {
        CompactionStatsSnapshot {
            compactions_completed: self.compactions_completed.load(Ordering::Relaxed),
            bytes_compacted: self.bytes_compacted.load(Ordering::Relaxed),
            total_compaction_time_ms: self.total_compaction_time_ms.load(Ordering::Relaxed),
            pending_compactions: self.pending_compactions.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactionStatsSnapshot {
    pub compactions_completed: u64,
    pub bytes_compacted: u64,
    pub total_compaction_time_ms: u64,
    pub pending_compactions: u64,
}

impl CompactionStatsSnapshot {
    pub fn avg_compaction_time_ms(&self) -> f64 {
        if self.compactions_completed > 0 {
            self.total_compaction_time_ms as f64 / self.compactions_completed as f64
        } else {
            0.0
        }
    }
}
