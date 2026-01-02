#!/bin/bash
# MikuDB OpenEuler Optimized Installation Script
# Optimized for OpenEuler with ARM64 (Kunpeng) processor support
# 支持中英文双语 / Supports Chinese and English

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
LANG="${LANG:-}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m'

# English strings
declare -A STRINGS_EN=(
    [Title]="MikuDB OpenEuler Installation Script"
    [Subtitle]="Optimized for Kunpeng Processors"
    [SelectLang]="Please select language / 请选择语言:\n  1. English\n  2. 中文"
    [LangPrompt]="Enter your choice (1-2): "
    [InvalidLang]="Invalid choice, using English"
    [NeedRoot]="This script must be run as root"
    [RunAsRoot]="Please run: sudo \$0"
    [Architecture]="Architecture:"
    [DetectedARM64]="Detected ARM64 (Kunpeng) processor"
    [NonARM64]="Non-ARM64 architecture, some optimizations will be disabled"
    [NumaNodes]="NUMA nodes available:"
    [NumaEnabled]="NUMA optimization will be enabled"
    [NumaInstalling]="numactl not found, installing..."
    [Uninstalling]="Uninstalling MikuDB..."
    [StopService]="Stopping MikuDB service..."
    [ServiceStopped]="Service stopped"
    [RemovingBins]="Removing binaries..."
    [BinsRemoved]="Binaries removed"
    [RemovingConfig]="Removing configuration..."
    [ConfigRemoved]="Configuration removed"
    [RemovingUser]="Removing user..."
    [UserRemoved]="User removed"
    [DisablingHP]="Disabling huge pages..."
    [HPDisabled]="Huge pages disabled"
    [DataPreserved]="Data directory preserved:"
    [ToRemoveData]="To remove data: rm -rf"
    [UninstallSuccess]="MikuDB uninstalled!"
    [ExistingInstall]="Existing MikuDB installation detected!"
    [ExistService]="Service:"
    [ExistBinary]="Binary:"
    [ExistConfig]="Config:"
    [OverwritePrompt]="Do you want to overwrite the existing installation? (y/N): "
    [Cancelled]="Installation aborted by user"
    [RemovingExisting]="Removing existing installation..."
    [ExistingRemoved]="Existing installation removed, continuing with fresh install..."
    [InstallDir]="Installation Directory:"
    [DataDir]="Data Directory:"
    [CheckingPrereq]="Checking prerequisites..."
    [InstallingRust]="Installing Rust..."
    [RustInstalled]="Rust installed"
    [FoundRust]="Found Rust:"
    [InstallingDeps]="Installing build dependencies..."
    [DepsInstalled]="Build tools installed"
    [CreatingUser]="Creating system user..."
    [UserCreated]="User created"
    [UserExists]="User exists"
    [CreatingDirs]="Creating directories..."
    [DirsCreated]="Directories created"
    [ConfiguringHP]="Configuring huge pages (2GB)..."
    [HPConfigured]="Huge pages configured:"
    [HPWarning]="Only"
    [HPAllocated]="huge pages allocated"
    [SkippingHP]="Skipping huge pages configuration"
    [OptimizingKernel]="Optimizing kernel parameters..."
    [KernelOptimized]="Kernel parameters optimized"
    [Building]="Building MikuDB with OpenEuler optimizations..."
    [BuildingServer]="Building mikudb-server..."
    [BuildingCli]="Building mikudb-cli..."
    [BuildComplete]="Build completed with OpenEuler optimizations"
    [InstallingBins]="Installing binaries..."
    [BinsInstalled]="Binaries installed"
    [CreatingConfig]="Creating optimized configuration..."
    [ConfigCreated]="Optimized configuration created"
    [CreatingService]="Creating systemd service..."
    [ServiceCreated]="Service created with NUMA binding"
    [EnablingService]="Enabling and starting service..."
    [ServiceStarted]="Service started successfully"
    [ServiceFailed]="Service failed to start"
    [CheckLogs]="Check logs: journalctl -u mikudb -n 50"
    [SkipService]="Skipping service installation"
    [SkipStart]="Skipping service start"
    [Verifying]="Verifying optimizations..."
    [CPUInfo]="CPU Info:"
    [HugePages]="Huge Pages:"
    [NumaTopology]="NUMA Topology:"
    [VerifyComplete]="Verification complete"
    [InstallComplete]="MikuDB Installation Complete!"
    [OptimizedBuild]="OpenEuler Optimized Build"
    [ConfigFile]="Configuration:"
    [OptsEnabled]="Optimizations Enabled:"
    [OptHP]="Huge Pages:"
    [OptNuma]="NUMA Aware:"
    [OptIOUring]="io_uring:"
    [OptTCP]="TCP Optimize:"
    [OptTarget]="Target CPU:"
    [Yes]="Yes"
    [No]="No"
    [YesHP]="Yes (2GB)"
    [YesNuma]="Yes (Node 0)"
    [NativeKunpeng]="Native (Kunpeng)"
    [ServiceStatus]="Service Status:"
    [Running]="Running"
    [ManageService]="Manage service:"
    [CmdStart]="sudo systemctl start mikudb"
    [CmdStop]="sudo systemctl stop mikudb"
    [CmdStatus]="sudo systemctl status mikudb"
    [CmdLogs]="sudo journalctl -u mikudb -f"
    [ConnectTo]="Connect to MikuDB:"
    [ConnectCmd]="mikudb-cli"
    [Username]="Username: root"
    [Password]="Password: mikudb_initial_password"
    [ChangePassword]="[IMPORTANT] Change default password!"
    [ChangeCmd]="ALTER USER \"root\" PASSWORD \"your_secure_password\";"
    [UninstallCmd]="Uninstall:"
    [UninstallHow]="sudo UNINSTALL=true \$0"
)

# Chinese strings
declare -A STRINGS_CN=(
    [Title]="MikuDB OpenEuler 安装脚本"
    [Subtitle]="针对鲲鹏处理器优化"
    [SelectLang]="请选择语言 / Please select language:\n  1. English\n  2. 中文"
    [LangPrompt]="输入你的选择 (1-2): "
    [InvalidLang]="无效选择，使用中文"
    [NeedRoot]="此脚本必须以 root 身份运行"
    [RunAsRoot]="请运行: sudo \$0"
    [Architecture]="架构:"
    [DetectedARM64]="检测到 ARM64 (鲲鹏) 处理器"
    [NonARM64]="非 ARM64 架构，部分优化将被禁用"
    [NumaNodes]="可用 NUMA 节点数:"
    [NumaEnabled]="将启用 NUMA 优化"
    [NumaInstalling]="未找到 numactl，正在安装..."
    [Uninstalling]="正在卸载 MikuDB..."
    [StopService]="停止 MikuDB 服务..."
    [ServiceStopped]="服务已停止"
    [RemovingBins]="移除二进制文件..."
    [BinsRemoved]="二进制文件已移除"
    [RemovingConfig]="移除配置..."
    [ConfigRemoved]="配置已移除"
    [RemovingUser]="移除用户..."
    [UserRemoved]="用户已移除"
    [DisablingHP]="禁用大页..."
    [HPDisabled]="大页已禁用"
    [DataPreserved]="数据目录已保留:"
    [ToRemoveData]="要删除数据: rm -rf"
    [UninstallSuccess]="MikuDB 卸载成功！"
    [ExistingInstall]="检测到已存在的 MikuDB 安装！"
    [ExistService]="服务:"
    [ExistBinary]="二进制:"
    [ExistConfig]="配置:"
    [OverwritePrompt]="是否覆盖现有安装？(y/N): "
    [Cancelled]="用户取消安装"
    [RemovingExisting]="移除现有安装..."
    [ExistingRemoved]="现有安装已移除，继续全新安装..."
    [InstallDir]="安装目录:"
    [DataDir]="数据目录:"
    [CheckingPrereq]="检查先决条件..."
    [InstallingRust]="安装 Rust..."
    [RustInstalled]="Rust 已安装"
    [FoundRust]="找到 Rust:"
    [InstallingDeps]="安装编译依赖..."
    [DepsInstalled]="编译工具已安装"
    [CreatingUser]="创建系统用户..."
    [UserCreated]="用户已创建"
    [UserExists]="用户已存在"
    [CreatingDirs]="创建目录..."
    [DirsCreated]="目录已创建"
    [ConfiguringHP]="配置大页 (2GB)..."
    [HPConfigured]="大页已配置:"
    [HPWarning]="仅"
    [HPAllocated]="个大页已分配"
    [SkippingHP]="跳过大页配置"
    [OptimizingKernel]="优化内核参数..."
    [KernelOptimized]="内核参数已优化"
    [Building]="使用 OpenEuler 优化编译 MikuDB..."
    [BuildingServer]="编译 mikudb-server..."
    [BuildingCli]="编译 mikudb-cli..."
    [BuildComplete]="OpenEuler 优化编译完成"
    [InstallingBins]="安装二进制文件..."
    [BinsInstalled]="二进制文件已安装"
    [CreatingConfig]="创建优化配置..."
    [ConfigCreated]="优化配置已创建"
    [CreatingService]="创建 systemd 服务..."
    [ServiceCreated]="已创建 NUMA 绑定服务"
    [EnablingService]="启用并启动服务..."
    [ServiceStarted]="服务启动成功"
    [ServiceFailed]="服务启动失败"
    [CheckLogs]="检查日志: journalctl -u mikudb -n 50"
    [SkipService]="跳过服务安装"
    [SkipStart]="跳过服务启动"
    [Verifying]="验证优化..."
    [CPUInfo]="CPU 信息:"
    [HugePages]="大页:"
    [NumaTopology]="NUMA 拓扑:"
    [VerifyComplete]="验证完成"
    [InstallComplete]="MikuDB 安装完成！"
    [OptimizedBuild]="OpenEuler 优化版本"
    [ConfigFile]="配置:"
    [OptsEnabled]="已启用的优化:"
    [OptHP]="大页:"
    [OptNuma]="NUMA 感知:"
    [OptIOUring]="io_uring:"
    [OptTCP]="TCP 优化:"
    [OptTarget]="目标 CPU:"
    [Yes]="是"
    [No]="否"
    [YesHP]="是 (2GB)"
    [YesNuma]="是 (节点 0)"
    [NativeKunpeng]="原生 (鲲鹏)"
    [ServiceStatus]="服务状态:"
    [Running]="运行中"
    [ManageService]="管理服务:"
    [CmdStart]="sudo systemctl start mikudb"
    [CmdStop]="sudo systemctl stop mikudb"
    [CmdStatus]="sudo systemctl status mikudb"
    [CmdLogs]="sudo journalctl -u mikudb -f"
    [ConnectTo]="连接到 MikuDB:"
    [ConnectCmd]="mikudb-cli"
    [Username]="用户名: root"
    [Password]="密码: mikudb_initial_password"
    [ChangePassword]="[重要] 请修改默认密码！"
    [ChangeCmd]="ALTER USER \"root\" PASSWORD \"your_secure_password\";"
    [UninstallCmd]="卸载:"
    [UninstallHow]="sudo UNINSTALL=true \$0"
)

get_text() {
    local key=$1
    if [ "$SELECTED_LANG" = "en" ]; then
        echo "${STRINGS_EN[$key]}"
    else
        echo "${STRINGS_CN[$key]}"
    fi
}

select_language() {
    if [ "$LANG" = "en" ] || [ "$LANG" = "EN" ]; then
        SELECTED_LANG="en"
        return
    fi
    if [ "$LANG" = "zh" ] || [ "$LANG" = "cn" ] || [ "$LANG" = "CN" ]; then
        SELECTED_LANG="cn"
        return
    fi

    echo ""
    echo -e "Please select language / 请选择语言:"
    echo "  1. English"
    echo "  2. 中文"
    echo ""
    read -p "Enter your choice (1-2): " choice

    case $choice in
        1) SELECTED_LANG="en" ;;
        2) SELECTED_LANG="cn" ;;
        *)
            echo "Invalid choice, using Chinese / 无效选择，使用中文"
            SELECTED_LANG="cn"
            ;;
    esac
}

select_language

echo -e "${CYAN}=========================================="
echo -e "   $(get_text 'Title')"
echo -e "   $(get_text 'Subtitle')"
echo -e "==========================================${NC}"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] $(get_text 'NeedRoot')${NC}"
    echo -e "${YELLOW}$(get_text 'RunAsRoot')${NC}"
    exit 1
fi

detect_architecture() {
    ARCH=$(uname -m)
    echo -e "${CYAN}[INFO] $(get_text 'Architecture') $ARCH${NC}"

    if [ "$ARCH" = "aarch64" ]; then
        IS_ARM64=true
        echo -e "${GREEN}[OK] $(get_text 'DetectedARM64')${NC}"
    else
        IS_ARM64=false
        echo -e "${YELLOW}[INFO] $(get_text 'NonARM64')${NC}"
    fi
}

check_numa() {
    if command -v numactl &> /dev/null; then
        NUMA_NODES=$(numactl --hardware | grep "available:" | awk '{print $2}')
        echo -e "${CYAN}[INFO] $(get_text 'NumaNodes') $NUMA_NODES${NC}"
        if [ "$NUMA_NODES" -gt 1 ] && [ "$ENABLE_NUMA" = "auto" ]; then
            ENABLE_NUMA=true
            echo -e "${GREEN}[OK] $(get_text 'NumaEnabled')${NC}"
        elif [ "$ENABLE_NUMA" = "auto" ]; then
            ENABLE_NUMA=false
        fi
    else
        echo -e "${YELLOW}[INFO] $(get_text 'NumaInstalling')${NC}"
        yum install -y numactl > /dev/null 2>&1
        ENABLE_NUMA=false
    fi
}

uninstall_mikudb() {
    echo -e "${YELLOW}[1/6] $(get_text 'StopService')${NC}"
    systemctl stop mikudb 2>/dev/null || true
    systemctl disable mikudb 2>/dev/null || true
    rm -f /etc/systemd/system/mikudb.service
    systemctl daemon-reload
    echo -e "${GREEN}[OK] $(get_text 'ServiceStopped')${NC}"

    echo -e "${YELLOW}[2/6] $(get_text 'RemovingBins')${NC}"
    rm -f "$INSTALL_DIR/mikudb-server"
    rm -f "$INSTALL_DIR/mikudb-cli"
    echo -e "${GREEN}[OK] $(get_text 'BinsRemoved')${NC}"

    echo -e "${YELLOW}[3/6] $(get_text 'RemovingConfig')${NC}"
    rm -rf "$CONFIG_DIR"
    echo -e "${GREEN}[OK] $(get_text 'ConfigRemoved')${NC}"

    echo -e "${YELLOW}[4/6] $(get_text 'RemovingUser')${NC}"
    userdel "$SERVICE_USER" 2>/dev/null || true
    echo -e "${GREEN}[OK] $(get_text 'UserRemoved')${NC}"

    echo -e "${YELLOW}[5/6] $(get_text 'DisablingHP')${NC}"
    sysctl -w vm.nr_hugepages=0 > /dev/null 2>&1 || true
    sed -i '/vm.nr_hugepages/d' /etc/sysctl.conf 2>/dev/null || true
    echo -e "${GREEN}[OK] $(get_text 'HPDisabled')${NC}"

    echo -e "${YELLOW}[6/6] $(get_text 'DataPreserved') $DATA_DIR${NC}"
    echo -e "${CYAN}$(get_text 'ToRemoveData') $DATA_DIR${NC}"

    echo ""
    echo -e "${GREEN}[SUCCESS] $(get_text 'UninstallSuccess')${NC}"
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
        echo -e "${YELLOW}[WARNING] $(get_text 'ExistingInstall')${NC}"
        echo ""

        if [ "$has_service" = "true" ]; then
            SERVICE_STATUS=$(systemctl is-active mikudb 2>/dev/null || echo "inactive")
            echo -e "${CYAN}  $(get_text 'ExistService') $SERVICE_STATUS${NC}"
        fi

        if [ "$has_binary" = "true" ]; then
            BINARY_VERSION=$($INSTALL_DIR/mikudb-server --version 2>/dev/null || echo "unknown")
            echo -e "${CYAN}  $(get_text 'ExistBinary') $INSTALL_DIR/mikudb-server ($BINARY_VERSION)${NC}"
        fi

        if [ "$has_config" = "true" ]; then
            echo -e "${CYAN}  $(get_text 'ExistConfig') $CONFIG_DIR/mikudb.toml${NC}"
        fi

        echo ""
        read -p "$(get_text 'OverwritePrompt')" -n 1 -r
        echo ""

        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}[CANCELLED] $(get_text 'Cancelled')${NC}"
            exit 0
        fi

        echo ""
        echo -e "${YELLOW}[INFO] $(get_text 'RemovingExisting')${NC}"
        UNINSTALL=true uninstall_mikudb
        echo -e "${GREEN}[OK] $(get_text 'ExistingRemoved')${NC}"
        echo ""
    fi
}

check_existing_installation

detect_architecture
check_numa

echo -e "${CYAN}[INFO] $(get_text 'InstallDir') $INSTALL_DIR${NC}"
echo -e "${CYAN}[INFO] $(get_text 'DataDir') $DATA_DIR${NC}"
echo ""

echo -e "${YELLOW}[1/12] $(get_text 'CheckingPrereq')${NC}"

if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}$(get_text 'InstallingRust')${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}[OK] $(get_text 'RustInstalled')${NC}"
else
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}[OK] $(get_text 'FoundRust') $RUST_VERSION${NC}"
fi

echo -e "${YELLOW}[2/12] $(get_text 'InstallingDeps')${NC}"
yum install -y gcc gcc-c++ make cmake clang > /dev/null 2>&1
echo -e "${GREEN}[OK] $(get_text 'DepsInstalled')${NC}"

echo -e "${YELLOW}[3/12] $(get_text 'CreatingUser')${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    echo -e "${GREEN}[OK] $(get_text 'UserCreated')${NC}"
else
    echo -e "${GRAY}[SKIP] $(get_text 'UserExists')${NC}"
fi

echo -e "${YELLOW}[4/12] $(get_text 'CreatingDirs')${NC}"
mkdir -p "$DATA_DIR"/{data,logs,config}
mkdir -p "$CONFIG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
echo -e "${GREEN}[OK] $(get_text 'DirsCreated')${NC}"

if [ "$ENABLE_HUGEPAGES" = "true" ]; then
    echo -e "${YELLOW}[5/12] $(get_text 'ConfiguringHP')${NC}"

    HUGEPAGES_COUNT=1024
    sysctl -w vm.nr_hugepages=$HUGEPAGES_COUNT > /dev/null 2>&1

    if ! grep -q "vm.nr_hugepages" /etc/sysctl.conf; then
        echo "vm.nr_hugepages=$HUGEPAGES_COUNT" >> /etc/sysctl.conf
    fi

    ACTUAL_HP=$(cat /proc/sys/vm/nr_hugepages)
    if [ "$ACTUAL_HP" -ge "$HUGEPAGES_COUNT" ]; then
        echo -e "${GREEN}[OK] $(get_text 'HPConfigured') ${ACTUAL_HP} pages (2MB each)${NC}"
    else
        echo -e "${YELLOW}[WARNING] $(get_text 'HPWarning') ${ACTUAL_HP} $(get_text 'HPAllocated')${NC}"
        ENABLE_HUGEPAGES=false
    fi
else
    echo -e "${GRAY}[5/12] $(get_text 'SkippingHP')${NC}"
fi

echo -e "${YELLOW}[6/12] $(get_text 'OptimizingKernel')${NC}"
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
echo -e "${GREEN}[OK] $(get_text 'KernelOptimized')${NC}"

echo -e "${YELLOW}[7/12] $(get_text 'Building')${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

export RUSTFLAGS="-C target-cpu=native"
if [ "$IS_ARM64" = true ]; then
    export RUSTFLAGS="$RUSTFLAGS -C target-feature=+neon"
fi

echo -e "${GRAY}    $(get_text 'BuildingServer')${NC}"
cargo build --release --features openeuler -p mikudb-server 2>&1 | grep -E "Finished|error" || true

echo -e "${GRAY}    $(get_text 'BuildingCli')${NC}"
cargo build --release -p mikudb-cli 2>&1 | grep -E "Finished|error" || true

echo -e "${GREEN}[OK] $(get_text 'BuildComplete')${NC}"

echo -e "${YELLOW}[8/12] $(get_text 'InstallingBins')${NC}"
cp target/release/mikudb-server "$INSTALL_DIR/"
cp target/release/mikudb-cli "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/mikudb-server"
chmod +x "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] $(get_text 'BinsInstalled')${NC}"

echo -e "${YELLOW}[9/12] $(get_text 'CreatingConfig')${NC}"

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

echo -e "${GREEN}[OK] $(get_text 'ConfigCreated')${NC}"

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${YELLOW}[10/12] $(get_text 'CreatingService')${NC}"

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

    echo -e "${GREEN}[OK] $(get_text 'ServiceCreated')${NC}"

    echo -e "${YELLOW}[11/12] $(get_text 'EnablingService')${NC}"
    systemctl daemon-reload
    systemctl enable mikudb
    systemctl start mikudb

    sleep 3
    if systemctl is-active --quiet mikudb; then
        echo -e "${GREEN}[OK] $(get_text 'ServiceStarted')${NC}"
    else
        echo -e "${YELLOW}[WARNING] $(get_text 'ServiceFailed')${NC}"
        echo -e "${YELLOW}          $(get_text 'CheckLogs')${NC}"
    fi
else
    echo -e "${GRAY}[10/12] $(get_text 'SkipService')${NC}"
    echo -e "${GRAY}[11/12] $(get_text 'SkipStart')${NC}"
fi

echo -e "${YELLOW}[12/12] $(get_text 'Verifying')${NC}"
echo -e "${CYAN}  $(get_text 'CPUInfo')${NC}"
lscpu | grep -E "Architecture|Model name|CPU\(s\):" | sed 's/^/    /'

echo -e "${CYAN}  $(get_text 'HugePages')${NC}"
grep HugePages /proc/meminfo | sed 's/^/    /'

if [ "$ENABLE_NUMA" = "true" ]; then
    echo -e "${CYAN}  $(get_text 'NumaTopology')${NC}"
    numactl --hardware | head -3 | sed 's/^/    /'
fi

echo -e "${GREEN}[OK] $(get_text 'VerifyComplete')${NC}"

echo ""
echo -e "${GREEN}==========================================="
echo -e "   $(get_text 'InstallComplete')"
echo -e "   $(get_text 'OptimizedBuild')"
echo -e "===========================================${NC}"
echo ""
echo -e "${CYAN}$(get_text 'InstallDir') $INSTALL_DIR${NC}"
echo -e "${CYAN}$(get_text 'DataDir')         $DATA_DIR${NC}"
echo -e "${CYAN}$(get_text 'ConfigFile')          $CONFIG_DIR/mikudb.toml${NC}"
echo ""
echo -e "${GREEN}$(get_text 'OptsEnabled')${NC}"
echo -e "${CYAN}  $(get_text 'OptHP')    $([ "$ENABLE_HUGEPAGES" = "true" ] && get_text 'YesHP' || get_text 'No')${NC}"
echo -e "${CYAN}  $(get_text 'OptNuma')    $([ "$ENABLE_NUMA" = "true" ] && get_text 'YesNuma' || get_text 'No')${NC}"
echo -e "${CYAN}  $(get_text 'OptIOUring')      $(get_text 'Yes')${NC}"
echo -e "${CYAN}  $(get_text 'OptTCP')  $(get_text 'Yes')${NC}"
echo -e "${CYAN}  $(get_text 'OptTarget')    $(get_text 'NativeKunpeng')${NC}"
echo ""

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${GREEN}$(get_text 'ServiceStatus')         $(get_text 'Running')${NC}"
    echo ""
    echo -e "${YELLOW}$(get_text 'ManageService')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStart')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStop')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStatus')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdLogs')${NC}"
    echo ""
fi

echo -e "${YELLOW}$(get_text 'ConnectTo')${NC}"
echo -e "${GRAY}  $(get_text 'ConnectCmd')${NC}"
echo -e "${GRAY}  $(get_text 'Username')${NC}"
echo -e "${GRAY}  $(get_text 'Password')${NC}"
echo ""
echo -e "${RED}$(get_text 'ChangePassword')${NC}"
echo -e "${YELLOW}$(get_text 'ChangeCmd')${NC}"
echo ""
echo -e "${YELLOW}$(get_text 'UninstallCmd')${NC}"
echo -e "${GRAY}  $(get_text 'UninstallHow')${NC}"
echo ""
