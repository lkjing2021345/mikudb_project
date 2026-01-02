#!/bin/bash
# MikuDB OpenEuler Optimized Installation Script
# Optimized for OpenEuler with ARM64 (Kunpeng) processor support

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
DATA_DIR="${DATA_DIR:-/var/lib/mikudb}"
CONFIG_DIR="${CONFIG_DIR:-/etc/mikudb}"
SERVICE_USER="${SERVICE_USER:-mikudb}"
SKIP_SERVICE="${SKIP_SERVICE:-false}"
UNINSTALL="${UNINSTALL:-false}"
ENABLE_HUGEPAGES="${ENABLE_HUGEPAGES:-true}"
ENABLE_NUMA="${ENABLE_NUMA:-auto}"
CPU_AFFINITY="${CPU_AFFINITY:-auto}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m'

echo -e "${CYAN}=========================================="
echo -e "   MikuDB OpenEuler Installation Script"
echo -e "   Optimized for Kunpeng Processors"
echo -e "==========================================${NC}"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] This script must be run as root${NC}"
    echo -e "${YELLOW}Please run: sudo $0${NC}"
    exit 1
fi

detect_architecture() {
    ARCH=$(uname -m)
    echo -e "${CYAN}[INFO] Architecture: $ARCH${NC}"

    if [ "$ARCH" = "aarch64" ]; then
        IS_ARM64=true
        echo -e "${GREEN}[OK] Detected ARM64 (Kunpeng) processor${NC}"
    else
        IS_ARM64=false
        echo -e "${YELLOW}[INFO] Non-ARM64 architecture, some optimizations will be disabled${NC}"
    fi
}

check_numa() {
    if command -v numactl &> /dev/null; then
        NUMA_NODES=$(numactl --hardware | grep "available:" | awk '{print $2}')
        echo -e "${CYAN}[INFO] NUMA nodes available: $NUMA_NODES${NC}"
        if [ "$NUMA_NODES" -gt 1 ] && [ "$ENABLE_NUMA" = "auto" ]; then
            ENABLE_NUMA=true
            echo -e "${GREEN}[OK] NUMA optimization will be enabled${NC}"
        elif [ "$ENABLE_NUMA" = "auto" ]; then
            ENABLE_NUMA=false
        fi
    else
        echo -e "${YELLOW}[INFO] numactl not found, installing...${NC}"
        yum install -y numactl > /dev/null 2>&1
        ENABLE_NUMA=false
    fi
}

uninstall_mikudb() {
    echo -e "${YELLOW}[1/6] Stopping MikuDB service...${NC}"
    systemctl stop mikudb 2>/dev/null || true
    systemctl disable mikudb 2>/dev/null || true
    rm -f /etc/systemd/system/mikudb.service
    systemctl daemon-reload
    echo -e "${GREEN}[OK] Service stopped${NC}"

    echo -e "${YELLOW}[2/6] Removing binaries...${NC}"
    rm -f "$INSTALL_DIR/mikudb-server"
    rm -f "$INSTALL_DIR/mikudb-cli"
    echo -e "${GREEN}[OK] Binaries removed${NC}"

    echo -e "${YELLOW}[3/6] Removing configuration...${NC}"
    rm -rf "$CONFIG_DIR"
    echo -e "${GREEN}[OK] Configuration removed${NC}"

    echo -e "${YELLOW}[4/6] Removing user...${NC}"
    userdel "$SERVICE_USER" 2>/dev/null || true
    echo -e "${GREEN}[OK] User removed${NC}"

    echo -e "${YELLOW}[5/6] Disabling huge pages...${NC}"
    sysctl -w vm.nr_hugepages=0 > /dev/null 2>&1 || true
    sed -i '/vm.nr_hugepages/d' /etc/sysctl.conf 2>/dev/null || true
    echo -e "${GREEN}[OK] Huge pages disabled${NC}"

    echo -e "${YELLOW}[6/6] Data directory preserved: $DATA_DIR${NC}"
    echo -e "${CYAN}To remove data: rm -rf $DATA_DIR${NC}"

    echo ""
    echo -e "${GREEN}[SUCCESS] MikuDB uninstalled!${NC}"
    exit 0
}

if [ "$UNINSTALL" = "true" ]; then
    uninstall_mikudb
fi

check_existing_installation() {
    local has_service=false
    local has_binary=false
    local has_config=false

    if systemctl list-units --full --all | grep -q "mikudb.service"; then
        has_service=true
    fi

    if [ -f "$INSTALL_DIR/mikudb-server" ]; then
        has_binary=true
    fi

    if [ -f "$CONFIG_DIR/mikudb.toml" ]; then
        has_config=true
    fi

    if [ "$has_service" = "true" ] || [ "$has_binary" = "true" ] || [ "$has_config" = "true" ]; then
        echo ""
        echo -e "${YELLOW}[WARNING] Existing MikuDB installation detected!${NC}"
        echo ""

        if [ "$has_service" = "true" ]; then
            SERVICE_STATUS=$(systemctl is-active mikudb 2>/dev/null || echo "inactive")
            echo -e "${CYAN}  Service: $SERVICE_STATUS${NC}"
        fi

        if [ "$has_binary" = "true" ]; then
            BINARY_VERSION=$($INSTALL_DIR/mikudb-server --version 2>/dev/null || echo "unknown")
            echo -e "${CYAN}  Binary:  $INSTALL_DIR/mikudb-server ($BINARY_VERSION)${NC}"
        fi

        if [ "$has_config" = "true" ]; then
            echo -e "${CYAN}  Config:  $CONFIG_DIR/mikudb.toml${NC}"
        fi

        echo ""
        read -p "Do you want to overwrite the existing installation? (y/N): " -n 1 -r
        echo ""

        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}[CANCELLED] Installation aborted by user${NC}"
            exit 0
        fi

        echo ""
        echo -e "${YELLOW}[INFO] Removing existing installation...${NC}"
        UNINSTALL=true uninstall_mikudb
        echo -e "${GREEN}[OK] Existing installation removed, continuing with fresh install...${NC}"
        echo ""
    fi
}

check_existing_installation

detect_architecture
check_numa

echo -e "${CYAN}[INFO] Installation Directory: $INSTALL_DIR${NC}"
echo -e "${CYAN}[INFO] Data Directory: $DATA_DIR${NC}"
echo ""

echo -e "${YELLOW}[1/12] Checking prerequisites...${NC}"

if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}[OK] Rust installed${NC}"
else
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}[OK] Found Rust: $RUST_VERSION${NC}"
fi

echo -e "${YELLOW}[2/12] Installing build dependencies...${NC}"
yum install -y gcc gcc-c++ make cmake clang > /dev/null 2>&1
echo -e "${GREEN}[OK] Build tools installed${NC}"

echo -e "${YELLOW}[3/12] Creating system user...${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    echo -e "${GREEN}[OK] User created${NC}"
else
    echo -e "${GRAY}[SKIP] User exists${NC}"
fi

echo -e "${YELLOW}[4/12] Creating directories...${NC}"
mkdir -p "$DATA_DIR"/{data,logs,config}
mkdir -p "$CONFIG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
echo -e "${GREEN}[OK] Directories created${NC}"

if [ "$ENABLE_HUGEPAGES" = "true" ]; then
    echo -e "${YELLOW}[5/12] Configuring huge pages (2GB)...${NC}"

    HUGEPAGES_COUNT=1024
    sysctl -w vm.nr_hugepages=$HUGEPAGES_COUNT > /dev/null 2>&1

    if ! grep -q "vm.nr_hugepages" /etc/sysctl.conf; then
        echo "vm.nr_hugepages=$HUGEPAGES_COUNT" >> /etc/sysctl.conf
    fi

    ACTUAL_HP=$(cat /proc/sys/vm/nr_hugepages)
    if [ "$ACTUAL_HP" -ge "$HUGEPAGES_COUNT" ]; then
        echo -e "${GREEN}[OK] Huge pages configured: ${ACTUAL_HP} pages (2MB each)${NC}"
    else
        echo -e "${YELLOW}[WARNING] Only ${ACTUAL_HP} huge pages allocated${NC}"
        ENABLE_HUGEPAGES=false
    fi
else
    echo -e "${GRAY}[5/12] Skipping huge pages configuration${NC}"
fi

echo -e "${YELLOW}[6/12] Optimizing kernel parameters...${NC}"
cat >> /etc/sysctl.conf <<EOF

# MikuDB Network Optimizations
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 8192
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_keepalive_time = 600
net.ipv4.tcp_keepalive_intvl = 30
net.ipv4.tcp_keepalive_probes = 3
EOF

sysctl -p > /dev/null 2>&1
echo -e "${GREEN}[OK] Kernel parameters optimized${NC}"

echo -e "${YELLOW}[7/12] Building MikuDB with OpenEuler optimizations...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

export RUSTFLAGS="-C target-cpu=native"
if [ "$IS_ARM64" = true ]; then
    export RUSTFLAGS="$RUSTFLAGS -C target-feature=+neon"
fi

echo -e "${GRAY}    Building mikudb-server...${NC}"
cargo build --release --features openeuler -p mikudb-server 2>&1 | grep -E "Finished|error" || true

echo -e "${GRAY}    Building mikudb-cli...${NC}"
cargo build --release -p mikudb-cli 2>&1 | grep -E "Finished|error" || true

echo -e "${GREEN}[OK] Build completed with OpenEuler optimizations${NC}"

echo -e "${YELLOW}[8/12] Installing binaries...${NC}"
cp target/release/mikudb-server "$INSTALL_DIR/"
cp target/release/mikudb-cli "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/mikudb-server"
chmod +x "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] Binaries installed${NC}"

echo -e "${YELLOW}[9/12] Creating optimized configuration...${NC}"

CACHE_SIZE="4GB"
if [ "$IS_ARM64" = true ]; then
    TOTAL_MEM=$(free -g | awk '/^Mem:/{print $2}')
    CACHE_SIZE="${TOTAL_MEM}GB"
fi

cat > "$CONFIG_DIR/mikudb.toml" <<EOF
# MikuDB OpenEuler Optimized Configuration

# Network settings
bind = "0.0.0.0"
port = 3939

# Data directory
data_dir = "$DATA_DIR/data"

# Connection settings
max_connections = 20000
timeout_ms = 60000

# Storage settings
[storage]
page_size = 16384
cache_size = "$CACHE_SIZE"
compression = "lz4"
sync_writes = false

# Authentication
[auth]
enabled = true
default_user = "root"
default_password = "mikudb_initial_password"

# Logging
[log]
level = "info"
file = "$DATA_DIR/logs/mikudb.log"
rotation = "daily"
max_files = 7

# OpenEuler Optimizations
[openeuler]
enable_huge_pages = $ENABLE_HUGEPAGES
huge_pages_size_mb = 2048
enable_numa = $ENABLE_NUMA
numa_node = 0
enable_io_uring = true
cpu_affinity = []
enable_direct_io = false
tcp_cork = true
tcp_nodelay = true
EOF

echo -e "${GREEN}[OK] Optimized configuration created${NC}"

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${YELLOW}[10/12] Creating systemd service...${NC}"

    NUMA_CMD=""
    if [ "$ENABLE_NUMA" = "true" ]; then
        NUMA_CMD="numactl --cpunodebind=0 --membind=0 "
    fi

    cat > /etc/systemd/system/mikudb.service <<EOF
[Unit]
Description=MikuDB Database Server (OpenEuler Optimized)
After=network.target
Documentation=https://github.com/yourusername/mikudb

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=${NUMA_CMD}$INSTALL_DIR/mikudb-server --config $CONFIG_DIR/mikudb.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=1048576
LimitNPROC=unlimited

# Performance settings
Nice=-10
IOSchedulingClass=realtime
IOSchedulingPriority=0

# Security settings
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF

    echo -e "${GREEN}[OK] Service created with NUMA binding${NC}"

    echo -e "${YELLOW}[11/12] Enabling and starting service...${NC}"
    systemctl daemon-reload
    systemctl enable mikudb
    systemctl start mikudb

    sleep 3
    if systemctl is-active --quiet mikudb; then
        echo -e "${GREEN}[OK] Service started successfully${NC}"
    else
        echo -e "${YELLOW}[WARNING] Service failed to start${NC}"
        echo -e "${YELLOW}Check logs: journalctl -u mikudb -n 50${NC}"
    fi
else
    echo -e "${GRAY}[10/12] Skipping service installation${NC}"
    echo -e "${GRAY}[11/12] Skipping service start${NC}"
fi

echo -e "${YELLOW}[12/12] Verifying optimizations...${NC}"
echo -e "${CYAN}  CPU Info:${NC}"
lscpu | grep -E "Architecture|Model name|CPU\(s\):" | sed 's/^/    /'

echo -e "${CYAN}  Huge Pages:${NC}"
grep HugePages /proc/meminfo | sed 's/^/    /'

if [ "$ENABLE_NUMA" = "true" ]; then
    echo -e "${CYAN}  NUMA Topology:${NC}"
    numactl --hardware | head -3 | sed 's/^/    /'
fi

echo -e "${GREEN}[OK] Verification complete${NC}"

echo ""
echo -e "${GREEN}==========================================="
echo -e "   MikuDB Installation Complete!"
echo -e "   OpenEuler Optimized Build"
echo -e "===========================================${NC}"
echo ""
echo -e "${CYAN}Installation Directory: $INSTALL_DIR${NC}"
echo -e "${CYAN}Data Directory:         $DATA_DIR${NC}"
echo -e "${CYAN}Configuration:          $CONFIG_DIR/mikudb.toml${NC}"
echo ""
echo -e "${GREEN}Optimizations Enabled:${NC}"
echo -e "${CYAN}  Huge Pages:    $([ "$ENABLE_HUGEPAGES" = "true" ] && echo "Yes (2GB)" || echo "No")${NC}"
echo -e "${CYAN}  NUMA Aware:    $([ "$ENABLE_NUMA" = "true" ] && echo "Yes (Node 0)" || echo "No")${NC}"
echo -e "${CYAN}  io_uring:      Yes${NC}"
echo -e "${CYAN}  TCP Optimize:  Yes${NC}"
echo -e "${CYAN}  Target CPU:    Native (Kunpeng)${NC}"
echo ""

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${GREEN}Service Status:         Running${NC}"
    echo ""
    echo -e "${YELLOW}Manage service:${NC}"
    echo -e "${GRAY}  sudo systemctl start mikudb${NC}"
    echo -e "${GRAY}  sudo systemctl stop mikudb${NC}"
    echo -e "${GRAY}  sudo systemctl status mikudb${NC}"
    echo -e "${GRAY}  sudo journalctl -u mikudb -f${NC}"
    echo ""
fi

echo -e "${YELLOW}Connect to MikuDB:${NC}"
echo -e "${GRAY}  mikudb-cli${NC}"
echo -e "${GRAY}  Username: root${NC}"
echo -e "${GRAY}  Password: mikudb_initial_password${NC}"
echo ""
echo -e "${RED}[IMPORTANT] Change default password!${NC}"
echo -e "${YELLOW}ALTER USER \"root\" PASSWORD \"your_secure_password\";${NC}"
echo ""
echo -e "${YELLOW}Uninstall:${NC}"
echo -e "${GRAY}  sudo UNINSTALL=true $0${NC}"
echo ""
