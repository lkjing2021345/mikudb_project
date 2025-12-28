#!/bin/bash
set -e

INSTALL_DIR="/usr/local"

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
        log_error "Please run as root (use sudo)"
        exit 1
    fi
}

install_server() {
    log_info "Installing MikuDB server..."

    if [ -f "./target/release/mikudb-server" ]; then
        cp ./target/release/mikudb-server "$INSTALL_DIR/bin/"
        chmod 755 "$INSTALL_DIR/bin/mikudb-server"
        log_info "MikuDB server installed to $INSTALL_DIR/bin/mikudb-server"
        return 0
    else
        log_error "Server binary not found at ./target/release/mikudb-server"
        log_error "Please run 'cargo build --release -p mikudb-server' first"
        return 1
    fi
}

install_cli() {
    if [ -f "./target/release/mikudb-cli" ]; then
        echo
        read -p "Do you want to install mikudb-cli? [Y/n] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
            log_info "Installing MikuDB CLI..."
            cp ./target/release/mikudb-cli "$INSTALL_DIR/bin/"
            chmod 755 "$INSTALL_DIR/bin/mikudb-cli"
            log_info "MikuDB CLI installed to $INSTALL_DIR/bin/mikudb-cli"
            return 0
        else
            log_info "Skipping MikuDB CLI installation"
            return 1
        fi
    else
        log_warn "MikuDB CLI binary not found at ./target/release/mikudb-cli"
        log_warn "You can build and install it later with:"
        log_warn "  cargo build --release -p mikudb-cli"
        log_warn "  sudo cp target/release/mikudb-cli $INSTALL_DIR/bin/"
        return 1
    fi
}

main() {
    echo "========================================"
    echo "  MikuDB Installation Script"
    echo "========================================"
    echo

    check_root

    local server_installed=false
    local cli_installed=false

    install_server && server_installed=true || exit 1
    install_cli && cli_installed=true

    echo
    log_info "Installation completed!"
    echo

    if [ "$server_installed" = true ]; then
        echo "Server installed:"
        echo "  mikudb-server --version"
        echo
    fi

    if [ "$cli_installed" = true ]; then
        echo "CLI installed:"
        echo "  mikudb-cli --version"
        echo
        echo "To connect to server:"
        echo "  mikudb-cli --host localhost --port 3939 --user miku --password mikumiku3939"
    else
        echo "To install CLI later:"
        echo "  cargo build --release -p mikudb-cli"
        echo "  sudo cp target/release/mikudb-cli $INSTALL_DIR/bin/"
    fi
    echo
}

main "$@"
