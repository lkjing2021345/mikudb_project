#!/bin/bash
set -e

INSTALL_DIR="/usr/local"
CONFIG_DIR="/etc/mikudb"
DATA_DIR="/var/lib/mikudb"
LOG_DIR="/var/log/mikudb"
RUN_DIR="/var/run/mikudb"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "Please run as root"
        exit 1
    fi
}

detect_openeuler() {
    if [ -f /etc/os-release ]; then
        if grep -qi "openeuler" /etc/os-release; then
            log_info "OpenEuler detected"
            return 0
        fi
    fi
    log_warn "OpenEuler not detected, some optimizations may not apply"
    return 1
}

create_user() {
    if ! id "mikudb" &>/dev/null; then
        log_info "Creating mikudb user..."
        useradd -r -s /sbin/nologin -d "$DATA_DIR" mikudb
    else
        log_info "User mikudb already exists"
    fi
}

create_directories() {
    log_info "Creating directories..."

    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR/data"
    mkdir -p "$DATA_DIR/wal"
    mkdir -p "$LOG_DIR"
    mkdir -p "$RUN_DIR"

    chown -R mikudb:mikudb "$DATA_DIR"
    chown -R mikudb:mikudb "$LOG_DIR"
    chown -R mikudb:mikudb "$RUN_DIR"

    chmod 750 "$DATA_DIR"
    chmod 750 "$LOG_DIR"
    chmod 755 "$RUN_DIR"
}

install_binary() {
    log_info "Installing MikuDB binary..."

    if [ -f "./target/release/mikudb-server" ]; then
        cp ./target/release/mikudb-server "$INSTALL_DIR/bin/"
        chmod 755 "$INSTALL_DIR/bin/mikudb-server"
    else
        log_error "Binary not found. Please run 'cargo build --release' first"
        exit 1
    fi

    if [ -f "./target/release/mikudb-cli" ]; then
        cp ./target/release/mikudb-cli "$INSTALL_DIR/bin/"
        chmod 755 "$INSTALL_DIR/bin/mikudb-cli"
    fi
}

install_config() {
    log_info "Installing configuration..."

    if [ ! -f "$CONFIG_DIR/mikudb.toml" ]; then
        cat > "$CONFIG_DIR/mikudb.toml" << 'EOF'
[server]
bind = "0.0.0.0"
port = 3939
unix_socket = "/var/run/mikudb/mikudb.sock"
max_connections = 10000
timeout_ms = 30000

[storage]
page_size = 16384
cache_size = "1GB"
compression = "lz4"
wal_dir = "/var/lib/mikudb/wal"
sync_writes = false

[auth]
enabled = true
default_user = "miku"
default_password = "mikumiku3939"

[log]
level = "info"
file = "/var/log/mikudb/mikudb.log"
rotation = "daily"
max_files = 7

[openeuler]
enable_huge_pages = false
huge_pages_size_mb = 256
enable_numa = false
enable_io_uring = true
tcp_cork = true
tcp_nodelay = true
EOF
        log_info "Default configuration created at $CONFIG_DIR/mikudb.toml"
    else
        log_warn "Configuration file already exists, skipping"
    fi
}

install_systemd() {
    log_info "Installing systemd service..."

    if detect_openeuler; then
        cp ./systemd/mikudb-openeuler.service /etc/systemd/system/mikudb.service
    else
        cp ./systemd/mikudb.service /etc/systemd/system/mikudb.service
    fi

    cp ./scripts/openeuler/tune-kernel.sh "$INSTALL_DIR/bin/mikudb-tune-kernel.sh"
    chmod 755 "$INSTALL_DIR/bin/mikudb-tune-kernel.sh"

    systemctl daemon-reload
    systemctl enable mikudb

    log_info "Systemd service installed and enabled"
}

main() {
    echo "========================================"
    echo "  MikuDB Installation Script"
    echo "  Optimized for OpenEuler"
    echo "========================================"
    echo

    check_root
    detect_openeuler || true
    create_user
    create_directories
    install_binary
    install_config
    install_systemd

    echo
    log_info "Installation completed!"
    echo
    echo "To start MikuDB:"
    echo "  sudo systemctl start mikudb"
    echo
    echo "To check status:"
    echo "  sudo systemctl status mikudb"
    echo
    echo "To connect:"
    echo "  mikudb-cli --host localhost --port 3939 --user miku --password mikumiku3939"
    echo
}

main "$@"
