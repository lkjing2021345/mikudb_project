use crate::config::ServerConfig;
use crate::ServerResult;
use mikudb_common::platform::{linux, Platform};
use tracing::{info, warn};

pub fn apply_optimizations(config: &ServerConfig) -> ServerResult<()> {
    let platform = Platform::current();

    if platform.is_openeuler() {
        info!("Detected OpenEuler - applying optimizations");
        apply_openeuler_optimizations(config)?;
    } else if platform.supports_io_uring() {
        info!("Linux detected - applying standard optimizations");
        apply_linux_optimizations(config)?;
    }

    Ok(())
}

fn apply_openeuler_optimizations(config: &ServerConfig) -> ServerResult<()> {
    let oe_config = linux::openeuler::get_recommended_config();

    info!("OpenEuler config: {:?}", oe_config);

    if oe_config.is_kunpeng {
        info!("Kunpeng CPU detected - enabling ARM optimizations");
    }

    if config.openeuler.enable_huge_pages && oe_config.use_huge_pages {
        let size_mb = if config.openeuler.huge_pages_size_mb > 0 {
            config.openeuler.huge_pages_size_mb
        } else {
            256
        };

        match linux::enable_huge_pages(size_mb) {
            Ok(_) => info!("Huge pages enabled: {}MB", size_mb),
            Err(e) => warn!("Failed to enable huge pages: {}", e),
        }
    }

    if !config.openeuler.cpu_affinity.is_empty() {
        match linux::set_cpu_affinity(&config.openeuler.cpu_affinity) {
            Ok(_) => info!("CPU affinity set to {:?}", config.openeuler.cpu_affinity),
            Err(e) => warn!("Failed to set CPU affinity: {}", e),
        }
    }

    if config.openeuler.enable_numa {
        let numa_nodes = linux::get_numa_node_count();
        info!("NUMA nodes detected: {}", numa_nodes);

        if let Some(node) = config.openeuler.numa_node {
            if node < numa_nodes {
                info!("Binding to NUMA node {}", node);
            }
        }
    }

    if config.openeuler.enable_io_uring {
        if linux::check_io_uring_support() {
            info!("io_uring support enabled");
        } else {
            warn!("io_uring requested but not supported by kernel");
        }
    }

    Ok(())
}

fn apply_linux_optimizations(config: &ServerConfig) -> ServerResult<()> {
    if !config.openeuler.cpu_affinity.is_empty() {
        match linux::set_cpu_affinity(&config.openeuler.cpu_affinity) {
            Ok(_) => info!("CPU affinity set to {:?}", config.openeuler.cpu_affinity),
            Err(e) => warn!("Failed to set CPU affinity: {}", e),
        }
    }

    Ok(())
}

pub fn print_system_info() {
    let platform = Platform::current();
    info!("Platform: {:?}", platform);

    if platform.supports_io_uring() {
        let io_uring = linux::check_io_uring_support();
        info!("io_uring support: {}", io_uring);
    }

    if platform.supports_numa() {
        let numa_nodes = linux::get_numa_node_count();
        info!("NUMA nodes: {}", numa_nodes);
    }

    if let Ok(mem_info) = linux::get_memory_info() {
        info!(
            "Memory: total={}GB, available={}GB",
            mem_info.total / (1024 * 1024 * 1024),
            mem_info.available / (1024 * 1024 * 1024)
        );
        if mem_info.huge_pages_total > 0 {
            info!(
                "Huge pages: total={}, free={}",
                mem_info.huge_pages_total, mem_info.huge_pages_free
            );
        }
    }

    if platform.is_openeuler() {
        let oe_config = linux::openeuler::get_recommended_config();
        if oe_config.is_kunpeng {
            info!("Kunpeng CPU detected");
        }
        info!(
            "Recommended: cache={}GB, write_buffer={}MB",
            oe_config.recommended_cache_size / (1024 * 1024 * 1024),
            oe_config.recommended_write_buffer / (1024 * 1024)
        );
    }
}

pub fn tune_kernel_parameters() -> ServerResult<()> {
    use std::fs;

    let tunings = [
        ("/proc/sys/net/core/somaxconn", "65535"),
        ("/proc/sys/net/core/netdev_max_backlog", "65535"),
        ("/proc/sys/net/ipv4/tcp_max_syn_backlog", "65535"),
        ("/proc/sys/net/ipv4/tcp_fin_timeout", "10"),
        ("/proc/sys/net/ipv4/tcp_tw_reuse", "1"),
        ("/proc/sys/net/ipv4/tcp_keepalive_time", "60"),
        ("/proc/sys/net/ipv4/tcp_keepalive_intvl", "10"),
        ("/proc/sys/net/ipv4/tcp_keepalive_probes", "6"),
        ("/proc/sys/vm/swappiness", "10"),
        ("/proc/sys/vm/dirty_ratio", "40"),
        ("/proc/sys/vm/dirty_background_ratio", "10"),
    ];

    for (path, value) in tunings {
        match fs::write(path, value) {
            Ok(_) => info!("Set {} = {}", path, value),
            Err(e) => warn!("Failed to set {}: {} (may require root)", path, e),
        }
    }

    Ok(())
}
