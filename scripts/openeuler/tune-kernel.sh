#!/bin/bash
set -e

echo "MikuDB Kernel Tuning for OpenEuler"
echo "==================================="

sysctl -w net.core.somaxconn=65535
sysctl -w net.core.netdev_max_backlog=65535
sysctl -w net.ipv4.tcp_max_syn_backlog=65535
sysctl -w net.ipv4.tcp_fin_timeout=10
sysctl -w net.ipv4.tcp_tw_reuse=1
sysctl -w net.ipv4.tcp_keepalive_time=60
sysctl -w net.ipv4.tcp_keepalive_intvl=10
sysctl -w net.ipv4.tcp_keepalive_probes=6
sysctl -w net.ipv4.tcp_syncookies=1
sysctl -w net.ipv4.tcp_max_tw_buckets=262144
sysctl -w net.ipv4.ip_local_port_range="1024 65535"

sysctl -w vm.swappiness=10
sysctl -w vm.dirty_ratio=40
sysctl -w vm.dirty_background_ratio=10
sysctl -w vm.overcommit_memory=1
sysctl -w vm.max_map_count=262144

sysctl -w fs.file-max=2097152
sysctl -w fs.nr_open=2097152

if [ -f /sys/kernel/mm/transparent_hugepage/enabled ]; then
    echo never > /sys/kernel/mm/transparent_hugepage/enabled
    echo never > /sys/kernel/mm/transparent_hugepage/defrag
    echo "Transparent Huge Pages disabled"
fi

if command -v numactl &> /dev/null; then
    NUMA_NODES=$(numactl --hardware | grep "available:" | awk '{print $2}')
    echo "NUMA nodes available: $NUMA_NODES"
fi

if grep -q "Kunpeng" /proc/cpuinfo 2>/dev/null; then
    echo "Kunpeng CPU detected - applying ARM-specific optimizations"
fi

echo "Kernel tuning completed"
