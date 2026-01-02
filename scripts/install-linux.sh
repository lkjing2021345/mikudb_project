#!/bin/bash
# MikuDB Linux Installation Script
# Universal installation script for Ubuntu, Debian, CentOS, etc.

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
DATA_DIR="${DATA_DIR:-/var/lib/mikudb}"
CONFIG_DIR="${CONFIG_DIR:-/etc/mikudb}"
SERVICE_USER="${SERVICE_USER:-mikudb}"
SKIP_SERVICE="${SKIP_SERVICE:-false}"
UNINSTALL="${UNINSTALL:-false}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m'

echo -e "${CYAN}========================================"
echo -e "   MikuDB Linux Installation Script"
echo -e "========================================${NC}"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] This script must be run as root${NC}"
    echo -e "${YELLOW}Please run: sudo $0${NC}"
    exit 1
fi

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    elif [ -f /etc/redhat-release ]; then
        OS="centos"
    else
        OS="unknown"
    fi
    echo -e "${CYAN}[INFO] Detected OS: $OS $OS_VERSION${NC}"
}

uninstall_mikudb() {
    echo -e "${YELLOW}[1/5] Stopping MikuDB service...${NC}"
    systemctl stop mikudb 2>/dev/null || true
    systemctl disable mikudb 2>/dev/null || true
    rm -f /etc/systemd/system/mikudb.service
    systemctl daemon-reload
    echo -e "${GREEN}[OK] Service stopped and removed${NC}"

    echo -e "${YELLOW}[2/5] Removing binaries...${NC}"
    rm -f "$INSTALL_DIR/mikudb-server"
    rm -f "$INSTALL_DIR/mikudb-cli"
    echo -e "${GREEN}[OK] Binaries removed${NC}"

    echo -e "${YELLOW}[3/5] Removing configuration...${NC}"
    rm -rf "$CONFIG_DIR"
    echo -e "${GREEN}[OK] Configuration removed${NC}"

    echo -e "${YELLOW}[4/5] Removing user...${NC}"
    userdel "$SERVICE_USER" 2>/dev/null || true
    echo -e "${GREEN}[OK] User removed${NC}"

    echo -e "${YELLOW}[5/5] Data directory preserved: $DATA_DIR${NC}"
    echo -e "${CYAN}To remove data, manually run: rm -rf $DATA_DIR${NC}"

    echo ""
    echo -e "${GREEN}[SUCCESS] MikuDB uninstalled successfully!${NC}"
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

detect_os

echo -e "${CYAN}[INFO] Installation Directory: $INSTALL_DIR${NC}"
echo -e "${CYAN}[INFO] Data Directory: $DATA_DIR${NC}"
echo -e "${CYAN}[INFO] Config Directory: $CONFIG_DIR${NC}"
echo ""

echo -e "${YELLOW}[1/10] Checking prerequisites...${NC}"

if ! command -v rustc &> /dev/null; then
    echo -e "${RED}[ERROR] Rust is not installed${NC}"
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}[OK] Rust installed${NC}"
else
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}[OK] Found Rust: $RUST_VERSION${NC}"
fi

if ! command -v gcc &> /dev/null; then
    echo -e "${YELLOW}Installing build tools...${NC}"
    case $OS in
        ubuntu|debian)
            apt-get update -qq
            apt-get install -y build-essential cmake clang > /dev/null 2>&1
            ;;
        centos|rhel|fedora)
            yum install -y gcc gcc-c++ make cmake clang > /dev/null 2>&1
            ;;
        openeuler)
            yum install -y gcc gcc-c++ make cmake clang > /dev/null 2>&1
            ;;
    esac
    echo -e "${GREEN}[OK] Build tools installed${NC}"
fi

echo -e "${YELLOW}[2/10] Creating system user...${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    echo -e "${GREEN}[OK] User '$SERVICE_USER' created${NC}"
else
    echo -e "${GRAY}[SKIP] User already exists${NC}"
fi

echo -e "${YELLOW}[3/10] Creating directories...${NC}"
mkdir -p "$DATA_DIR"/{data,logs,config}
mkdir -p "$CONFIG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
echo -e "${GREEN}[OK] Directories created${NC}"

echo -e "${YELLOW}[4/10] Building MikuDB...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

echo -e "${GRAY}    Building mikudb-server...${NC}"
cargo build --release -p mikudb-server 2>&1 | grep -v "Compiling\|Finished" || true

echo -e "${GRAY}    Building mikudb-cli...${NC}"
cargo build --release -p mikudb-cli 2>&1 | grep -v "Compiling\|Finished" || true

echo -e "${GREEN}[OK] Build completed${NC}"

echo -e "${YELLOW}[5/10] Installing binaries...${NC}"
cp target/release/mikudb-server "$INSTALL_DIR/"
cp target/release/mikudb-cli "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/mikudb-server"
chmod +x "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] Binaries installed${NC}"

echo -e "${YELLOW}[6/10] Creating configuration...${NC}"
cat > "$CONFIG_DIR/mikudb.toml" <<EOF
# MikuDB Server Configuration

# Network settings
bind = "0.0.0.0"
port = 3939

# Data directory
data_dir = "$DATA_DIR/data"

# Connection settings
max_connections = 10000
timeout_ms = 30000

# Storage settings
[storage]
page_size = 16384
cache_size = "2GB"
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
EOF

echo -e "${GREEN}[OK] Configuration created${NC}"

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${YELLOW}[7/10] Creating systemd service...${NC}"
    cat > /etc/systemd/system/mikudb.service <<EOF
[Unit]
Description=MikuDB Database Server
After=network.target
Documentation=https://github.com/yourusername/mikudb

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=$INSTALL_DIR/mikudb-server --config $CONFIG_DIR/mikudb.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=65536

# Security settings
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF

    echo -e "${GREEN}[OK] Service created${NC}"

    echo -e "${YELLOW}[8/10] Enabling service...${NC}"
    systemctl daemon-reload
    systemctl enable mikudb
    echo -e "${GREEN}[OK] Service enabled${NC}"

    echo -e "${YELLOW}[9/10] Starting service...${NC}"
    systemctl start mikudb

    sleep 2
    if systemctl is-active --quiet mikudb; then
        echo -e "${GREEN}[OK] Service started successfully${NC}"
    else
        echo -e "${YELLOW}[WARNING] Service failed to start, check logs:${NC}"
        echo -e "${YELLOW}          journalctl -u mikudb -n 50${NC}"
    fi
else
    echo -e "${GRAY}[7/10] Skipping service installation${NC}"
    echo -e "${GRAY}[8/10] Skipping service enable${NC}"
    echo -e "${GRAY}[9/10] Skipping service start${NC}"
fi

echo -e "${YELLOW}[10/10] Setting permissions...${NC}"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
chmod 755 "$INSTALL_DIR/mikudb-server"
chmod 755 "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] Permissions set${NC}"

echo ""
echo -e "${GREEN}========================================"
echo -e "   MikuDB Installation Complete!"
echo -e "========================================${NC}"
echo ""
echo -e "${CYAN}Installation Directory: $INSTALL_DIR${NC}"
echo -e "${CYAN}Data Directory:         $DATA_DIR${NC}"
echo -e "${CYAN}Configuration File:     $CONFIG_DIR/mikudb.toml${NC}"
echo ""

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${GREEN}Service Status:         Running${NC}"
    echo -e "${CYAN}Service Name:           mikudb${NC}"
    echo ""
    echo -e "${YELLOW}Manage service:${NC}"
    echo -e "${GRAY}  Start:   sudo systemctl start mikudb${NC}"
    echo -e "${GRAY}  Stop:    sudo systemctl stop mikudb${NC}"
    echo -e "${GRAY}  Restart: sudo systemctl restart mikudb${NC}"
    echo -e "${GRAY}  Status:  sudo systemctl status mikudb${NC}"
    echo -e "${GRAY}  Logs:    sudo journalctl -u mikudb -f${NC}"
    echo ""
fi

echo -e "${YELLOW}Connect to MikuDB:${NC}"
echo -e "${GRAY}  mikudb-cli${NC}"
echo -e "${GRAY}  Username: root${NC}"
echo -e "${GRAY}  Password: mikudb_initial_password${NC}"
echo ""
echo -e "${RED}[IMPORTANT] Please change the default password!${NC}"
echo -e "${YELLOW}Use command: ALTER USER \"root\" PASSWORD \"your_secure_password\";${NC}"
echo ""
echo -e "${YELLOW}Uninstall:${NC}"
echo -e "${GRAY}  sudo UNINSTALL=true $0${NC}"
echo ""
