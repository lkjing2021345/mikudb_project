use crate::error::{MikuError, MikuResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    OpenEuler,
    Linux,
    Windows,
    MacOS,
    Unknown,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            if is_openeuler() {
                Platform::OpenEuler
            } else {
                Platform::Linux
            }
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            Platform::Unknown
        }
    }

    pub fn is_openeuler(&self) -> bool {
        matches!(self, Platform::OpenEuler)
    }

    pub fn supports_io_uring(&self) -> bool {
        matches!(self, Platform::OpenEuler | Platform::Linux)
    }

    pub fn supports_huge_pages(&self) -> bool {
        matches!(self, Platform::OpenEuler | Platform::Linux)
    }

    pub fn supports_numa(&self) -> bool {
        matches!(self, Platform::OpenEuler | Platform::Linux)
    }
}

#[cfg(target_os = "linux")]
fn is_openeuler() -> bool {
    use std::fs;
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        content.contains("openEuler") || content.contains("OpenEuler")
    } else {
        false
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use std::fs;
    use std::path::Path;

    pub fn enable_huge_pages(size_mb: usize) -> MikuResult<()> {
        let nr_pages = size_mb / 2;
        let path = "/proc/sys/vm/nr_hugepages";

        if !Path::new(path).exists() {
            return Err(MikuError::Platform(
                "Huge pages not supported on this system".to_string(),
            ));
        }

        fs::write(path, nr_pages.to_string()).map_err(|e| {
            MikuError::Platform(format!("Failed to enable huge pages: {}", e))
        })?;

        tracing::info!("Enabled {} huge pages ({}MB)", nr_pages, size_mb);
        Ok(())
    }

    pub fn get_numa_node_count() -> usize {
        let path = "/sys/devices/system/node";
        if let Ok(entries) = fs::read_dir(path) {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|s| s.starts_with("node"))
                        .unwrap_or(false)
                })
                .count()
        } else {
            1
        }
    }

    pub fn check_io_uring_support() -> bool {
        use std::process::Command;

        if let Ok(output) = Command::new("uname").arg("-r").output() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = version.trim().split('.').collect();
                if parts.len() >= 2 {
                    if let (Ok(major), Ok(minor)) = (
                        parts[0].parse::<u32>(),
                        parts[1].parse::<u32>(),
                    ) {
                        return major > 5 || (major == 5 && minor >= 1);
                    }
                }
            }
        }
        false
    }

    pub fn set_cpu_affinity(cpu_ids: &[usize]) -> MikuResult<()> {
        use nix::sched::{sched_setaffinity, CpuSet};
        use nix::unistd::Pid;

        let mut cpu_set = CpuSet::new();
        for &cpu_id in cpu_ids {
            cpu_set.set(cpu_id).map_err(|e| {
                MikuError::Platform(format!("Failed to set CPU {}: {}", cpu_id, e))
            })?;
        }

        sched_setaffinity(Pid::from_raw(0), &cpu_set).map_err(|e| {
            MikuError::Platform(format!("Failed to set CPU affinity: {}", e))
        })?;

        Ok(())
    }

    pub fn get_memory_info() -> MikuResult<MemoryInfo> {
        let content = fs::read_to_string("/proc/meminfo").map_err(|e| {
            MikuError::Platform(format!("Failed to read meminfo: {}", e))
        })?;

        let mut total = 0u64;
        let mut available = 0u64;
        let mut huge_pages_total = 0u64;
        let mut huge_pages_free = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "MemTotal:" => total = parts[1].parse().unwrap_or(0) * 1024,
                    "MemAvailable:" => available = parts[1].parse().unwrap_or(0) * 1024,
                    "HugePages_Total:" => huge_pages_total = parts[1].parse().unwrap_or(0),
                    "HugePages_Free:" => huge_pages_free = parts[1].parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        Ok(MemoryInfo {
            total,
            available,
            huge_pages_total,
            huge_pages_free,
        })
    }

    #[derive(Debug, Clone)]
    pub struct MemoryInfo {
        pub total: u64,
        pub available: u64,
        pub huge_pages_total: u64,
        pub huge_pages_free: u64,
    }

    pub mod openeuler {
        use super::*;

        pub fn detect_kunpeng_cpu() -> bool {
            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                content.contains("Kunpeng") || content.contains("HUAWEI")
            } else {
                false
            }
        }

        pub fn get_recommended_config() -> OpenEulerConfig {
            let is_kunpeng = detect_kunpeng_cpu();
            let numa_nodes = get_numa_node_count();
            let mem_info = get_memory_info().unwrap_or(MemoryInfo {
                total: 8 * 1024 * 1024 * 1024,
                available: 4 * 1024 * 1024 * 1024,
                huge_pages_total: 0,
                huge_pages_free: 0,
            });

            let cache_size = (mem_info.available / 4).min(16 * 1024 * 1024 * 1024);
            let write_buffer = if is_kunpeng { 128 * 1024 * 1024 } else { 64 * 1024 * 1024 };

            OpenEulerConfig {
                is_kunpeng,
                numa_nodes,
                recommended_cache_size: cache_size,
                recommended_write_buffer: write_buffer,
                use_huge_pages: mem_info.huge_pages_total > 0,
                use_io_uring: check_io_uring_support(),
                use_direct_io: true,
            }
        }

        #[derive(Debug, Clone)]
        pub struct OpenEulerConfig {
            pub is_kunpeng: bool,
            pub numa_nodes: usize,
            pub recommended_cache_size: u64,
            pub recommended_write_buffer: usize,
            pub use_huge_pages: bool,
            pub use_io_uring: bool,
            pub use_direct_io: bool,
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub mod linux {
    use super::*;

    pub fn enable_huge_pages(_size_mb: usize) -> MikuResult<()> {
        Err(MikuError::Platform(
            "Huge pages only supported on Linux".to_string(),
        ))
    }

    pub fn get_numa_node_count() -> usize {
        1
    }

    pub fn check_io_uring_support() -> bool {
        false
    }

    pub fn set_cpu_affinity(_cpu_ids: &[usize]) -> MikuResult<()> {
        Err(MikuError::Platform(
            "CPU affinity only supported on Linux".to_string(),
        ))
    }

    #[derive(Debug, Clone)]
    pub struct MemoryInfo {
        pub total: u64,
        pub available: u64,
        pub huge_pages_total: u64,
        pub huge_pages_free: u64,
    }

    pub fn get_memory_info() -> MikuResult<MemoryInfo> {
        Ok(MemoryInfo {
            total: 8 * 1024 * 1024 * 1024,
            available: 4 * 1024 * 1024 * 1024,
            huge_pages_total: 0,
            huge_pages_free: 0,
        })
    }

    pub mod openeuler {
        pub fn detect_kunpeng_cpu() -> bool {
            false
        }

        #[derive(Debug, Clone)]
        pub struct OpenEulerConfig {
            pub is_kunpeng: bool,
            pub numa_nodes: usize,
            pub recommended_cache_size: u64,
            pub recommended_write_buffer: usize,
            pub use_huge_pages: bool,
            pub use_io_uring: bool,
            pub use_direct_io: bool,
        }

        pub fn get_recommended_config() -> OpenEulerConfig {
            OpenEulerConfig {
                is_kunpeng: false,
                numa_nodes: 1,
                recommended_cache_size: 1024 * 1024 * 1024,
                recommended_write_buffer: 64 * 1024 * 1024,
                use_huge_pages: false,
                use_io_uring: false,
                use_direct_io: false,
            }
        }
    }
}
