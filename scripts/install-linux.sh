#!/bin/bash
# MikuDB Linux Installation Script
# Universal installation script for Ubuntu, Debian, CentOS, etc.
# 支持中英文双语 / Supports Chinese and English

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
DATA_DIR="${DATA_DIR:-/var/lib/mikudb}"
CONFIG_DIR="${CONFIG_DIR:-/etc/mikudb}"
SERVICE_USER="${SERVICE_USER:-mikudb}"
SKIP_SERVICE="${SKIP_SERVICE:-false}"
UNINSTALL="${UNINSTALL:-false}"
LANG="${LANG:-}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m'

# English strings
declare -A STRINGS_EN=(
    [Title]="MikuDB Linux Installation Script"
    [SelectLang]="Please select language / 请选择语言:\n  1. English\n  2. 中文"
    [LangPrompt]="Enter your choice (1-2): "
    [InvalidLang]="Invalid choice, using English"
    [NeedRoot]="This script must be run as root"
    [RunAsRoot]="Please run: sudo \$0"
    [DetectedOS]="Detected OS:"
    [Uninstalling]="Uninstalling MikuDB..."
    [StopService]="Stopping MikuDB service..."
    [ServiceStopped]="Service stopped and removed"
    [RemovingBins]="Removing binaries..."
    [BinsRemoved]="Binaries removed"
    [RemovingConfig]="Removing configuration..."
    [ConfigRemoved]="Configuration removed"
    [RemovingUser]="Removing user..."
    [UserRemoved]="User removed"
    [DataPreserved]="Data directory preserved:"
    [ToRemoveData]="To remove data, manually run: rm -rf"
    [UninstallSuccess]="MikuDB uninstalled successfully!"
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
    [ConfigDir]="Config Directory:"
    [CheckingPrereq]="Checking prerequisites..."
    [RustNotFound]="Rust is not installed"
    [InstallingRust]="Installing Rust..."
    [RustInstalled]="Rust installed"
    [FoundRust]="Found Rust:"
    [InstallingTools]="Installing build tools..."
    [ToolsInstalled]="Build tools installed"
    [CreatingUser]="Creating system user..."
    [UserCreated]="User created"
    [UserExists]="User already exists"
    [CreatingDirs]="Creating directories..."
    [DirsCreated]="Directories created"
    [Building]="Building MikuDB..."
    [BuildingServer]="Building mikudb-server..."
    [BuildingCli]="Building mikudb-cli..."
    [BuildComplete]="Build completed"
    [InstallingBins]="Installing binaries..."
    [BinsInstalled]="Binaries installed"
    [CreatingConfig]="Creating configuration..."
    [ConfigCreated]="Configuration created"
    [CreatingService]="Creating systemd service..."
    [ServiceCreated]="Service created"
    [EnablingService]="Enabling service..."
    [ServiceEnabled]="Service enabled"
    [StartingService]="Starting service..."
    [ServiceStarted]="Service started successfully"
    [ServiceFailed]="Service failed to start, check logs:"
    [SkipService]="Skipping service installation"
    [SkipEnable]="Skipping service enable"
    [SkipStart]="Skipping service start"
    [SettingPerms]="Setting permissions..."
    [PermsSet]="Permissions set"
    [InstallComplete]="MikuDB Installation Complete!"
    [ServiceStatus]="Service Status:"
    [ServiceRunning]="Running"
    [ServiceName]="Service Name:"
    [ManageService]="Manage service:"
    [CmdStart]="Start:   sudo systemctl start mikudb"
    [CmdStop]="Stop:    sudo systemctl stop mikudb"
    [CmdRestart]="Restart: sudo systemctl restart mikudb"
    [CmdStatus]="Status:  sudo systemctl status mikudb"
    [CmdLogs]="Logs:    sudo journalctl -u mikudb -f"
    [ConnectTo]="Connect to MikuDB:"
    [ConnectCmd]="mikudb-cli"
    [Username]="Username: root"
    [Password]="Password: mikudb_initial_password"
    [ChangePassword]="Please change the default password!"
    [ChangeCmd]="Use command: ALTER USER \"root\" PASSWORD \"your_secure_password\";"
    [UninstallCmd]="Uninstall:"
    [UninstallHow]="sudo UNINSTALL=true \$0"
)

# Chinese strings
declare -A STRINGS_CN=(
    [Title]="MikuDB Linux 安装脚本"
    [SelectLang]="请选择语言 / Please select language:\n  1. English\n  2. 中文"
    [LangPrompt]="输入你的选择 (1-2): "
    [InvalidLang]="无效选择，使用中文"
    [NeedRoot]="此脚本必须以 root 身份运行"
    [RunAsRoot]="请运行: sudo \$0"
    [DetectedOS]="检测到操作系统:"
    [Uninstalling]="正在卸载 MikuDB..."
    [StopService]="停止 MikuDB 服务..."
    [ServiceStopped]="服务已停止并移除"
    [RemovingBins]="移除二进制文件..."
    [BinsRemoved]="二进制文件已移除"
    [RemovingConfig]="移除配置..."
    [ConfigRemoved]="配置已移除"
    [RemovingUser]="移除用户..."
    [UserRemoved]="用户已移除"
    [DataPreserved]="数据目录已保留:"
    [ToRemoveData]="要删除数据，请手动运行: rm -rf"
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
    [ConfigDir]="配置目录:"
    [CheckingPrereq]="检查先决条件..."
    [RustNotFound]="Rust 未安装"
    [InstallingRust]="安装 Rust..."
    [RustInstalled]="Rust 已安装"
    [FoundRust]="找到 Rust:"
    [InstallingTools]="安装编译工具..."
    [ToolsInstalled]="编译工具已安装"
    [CreatingUser]="创建系统用户..."
    [UserCreated]="用户已创建"
    [UserExists]="用户已存在"
    [CreatingDirs]="创建目录..."
    [DirsCreated]="目录已创建"
    [Building]="编译 MikuDB..."
    [BuildingServer]="编译 mikudb-server..."
    [BuildingCli]="编译 mikudb-cli..."
    [BuildComplete]="编译完成"
    [InstallingBins]="安装二进制文件..."
    [BinsInstalled]="二进制文件已安装"
    [CreatingConfig]="创建配置..."
    [ConfigCreated]="配置已创建"
    [CreatingService]="创建 systemd 服务..."
    [ServiceCreated]="服务已创建"
    [EnablingService]="启用服务..."
    [ServiceEnabled]="服务已启用"
    [StartingService]="启动服务..."
    [ServiceStarted]="服务启动成功"
    [ServiceFailed]="服务启动失败，检查日志:"
    [SkipService]="跳过服务安装"
    [SkipEnable]="跳过服务启用"
    [SkipStart]="跳过服务启动"
    [SettingPerms]="设置权限..."
    [PermsSet]="权限已设置"
    [InstallComplete]="MikuDB 安装完成！"
    [ServiceStatus]="服务状态:"
    [ServiceRunning]="运行中"
    [ServiceName]="服务名称:"
    [ManageService]="管理服务:"
    [CmdStart]="启动:   sudo systemctl start mikudb"
    [CmdStop]="停止:   sudo systemctl stop mikudb"
    [CmdRestart]="重启:   sudo systemctl restart mikudb"
    [CmdStatus]="状态:   sudo systemctl status mikudb"
    [CmdLogs]="日志:   sudo journalctl -u mikudb -f"
    [ConnectTo]="连接到 MikuDB:"
    [ConnectCmd]="mikudb-cli"
    [Username]="用户名: root"
    [Password]="密码: mikudb_initial_password"
    [ChangePassword]="请修改默认密码！"
    [ChangeCmd]="使用命令: ALTER USER \"root\" PASSWORD \"your_secure_password\";"
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
echo -e "==========================================${NC}"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] $(get_text 'NeedRoot')${NC}"
    echo -e "${YELLOW}$(get_text 'RunAsRoot')${NC}"
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
    echo -e "${CYAN}[INFO] $(get_text 'DetectedOS') $OS $OS_VERSION${NC}"
}

uninstall_mikudb() {
    echo -e "${YELLOW}[1/5] $(get_text 'StopService')${NC}"
    systemctl stop mikudb 2>/dev/null || true
    systemctl disable mikudb 2>/dev/null || true
    rm -f /etc/systemd/system/mikudb.service
    systemctl daemon-reload
    echo -e "${GREEN}[OK] $(get_text 'ServiceStopped')${NC}"

    echo -e "${YELLOW}[2/5] $(get_text 'RemovingBins')${NC}"
    rm -f "$INSTALL_DIR/mikudb-server"
    rm -f "$INSTALL_DIR/mikudb-cli"
    echo -e "${GREEN}[OK] $(get_text 'BinsRemoved')${NC}"

    echo -e "${YELLOW}[3/5] $(get_text 'RemovingConfig')${NC}"
    rm -rf "$CONFIG_DIR"
    echo -e "${GREEN}[OK] $(get_text 'ConfigRemoved')${NC}"

    echo -e "${YELLOW}[4/5] $(get_text 'RemovingUser')${NC}"
    userdel "$SERVICE_USER" 2>/dev/null || true
    echo -e "${GREEN}[OK] $(get_text 'UserRemoved')${NC}"

    echo -e "${YELLOW}[5/5] $(get_text 'DataPreserved') $DATA_DIR${NC}"
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

detect_os

echo -e "${CYAN}[INFO] $(get_text 'InstallDir') $INSTALL_DIR${NC}"
echo -e "${CYAN}[INFO] $(get_text 'DataDir') $DATA_DIR${NC}"
echo -e "${CYAN}[INFO] $(get_text 'ConfigDir') $CONFIG_DIR${NC}"
echo ""

echo -e "${YELLOW}[1/10] $(get_text 'CheckingPrereq')${NC}"

if ! command -v rustc &> /dev/null; then
    echo -e "${RED}[ERROR] $(get_text 'RustNotFound')${NC}"
    echo -e "${YELLOW}$(get_text 'InstallingRust')${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}[OK] $(get_text 'RustInstalled')${NC}"
else
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}[OK] $(get_text 'FoundRust') $RUST_VERSION${NC}"
fi

if ! command -v gcc &> /dev/null; then
    echo -e "${YELLOW}$(get_text 'InstallingTools')${NC}"
    case $OS in
        ubuntu|debian)
            apt-get update -qq
            apt-get install -y build-essential cmake clang > /dev/null 2>&1
            ;;
        centos|rhel|fedora|openeuler)
            yum install -y gcc gcc-c++ make cmake clang > /dev/null 2>&1
            ;;
    esac
    echo -e "${GREEN}[OK] $(get_text 'ToolsInstalled')${NC}"
fi

echo -e "${YELLOW}[2/10] $(get_text 'CreatingUser')${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    echo -e "${GREEN}[OK] $(get_text 'UserCreated')${NC}"
else
    echo -e "${GRAY}[SKIP] $(get_text 'UserExists')${NC}"
fi

echo -e "${YELLOW}[3/10] $(get_text 'CreatingDirs')${NC}"
mkdir -p "$DATA_DIR"/{data,logs,config}
mkdir -p "$CONFIG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
echo -e "${GREEN}[OK] $(get_text 'DirsCreated')${NC}"

echo -e "${YELLOW}[4/10] $(get_text 'Building')${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

echo -e "${GRAY}    $(get_text 'BuildingServer')${NC}"
cargo build --release -p mikudb-server 2>&1 | grep -v "Compiling\|Finished" || true

echo -e "${GRAY}    $(get_text 'BuildingCli')${NC}"
cargo build --release -p mikudb-cli 2>&1 | grep -v "Compiling\|Finished" || true

echo -e "${GREEN}[OK] $(get_text 'BuildComplete')${NC}"

echo -e "${YELLOW}[5/10] $(get_text 'InstallingBins')${NC}"
cp target/release/mikudb-server "$INSTALL_DIR/"
cp target/release/mikudb-cli "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/mikudb-server"
chmod +x "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] $(get_text 'BinsInstalled')${NC}"

echo -e "${YELLOW}[6/10] $(get_text 'CreatingConfig')${NC}"
cat > "$CONFIG_DIR/mikudb.toml" <<EOF
# MikuDB Server Configuration

bind = "0.0.0.0"
port = 3939
data_dir = "$DATA_DIR/data"
max_connections = 10000
timeout_ms = 30000

[storage]
page_size = 16384
cache_size = "2GB"
compression = "lz4"
sync_writes = false

[auth]
enabled = true
default_user = "root"
default_password = "mikudb_initial_password"

[log]
level = "info"
file = "$DATA_DIR/logs/mikudb.log"
rotation = "daily"
max_files = 7
EOF

echo -e "${GREEN}[OK] $(get_text 'ConfigCreated')${NC}"

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${YELLOW}[7/10] $(get_text 'CreatingService')${NC}"
    cat > /etc/systemd/system/mikudb.service <<EOF
[Unit]
Description=MikuDB Database Server
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=$INSTALL_DIR/mikudb-server --config $CONFIG_DIR/mikudb.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=65536

ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF

    echo -e "${GREEN}[OK] $(get_text 'ServiceCreated')${NC}"

    echo -e "${YELLOW}[8/10] $(get_text 'EnablingService')${NC}"
    systemctl daemon-reload
    systemctl enable mikudb
    echo -e "${GREEN}[OK] $(get_text 'ServiceEnabled')${NC}"

    echo -e "${YELLOW}[9/10] $(get_text 'StartingService')${NC}"
    systemctl start mikudb

    sleep 2
    if systemctl is-active --quiet mikudb; then
        echo -e "${GREEN}[OK] $(get_text 'ServiceStarted')${NC}"
    else
        echo -e "${YELLOW}[WARNING] $(get_text 'ServiceFailed')${NC}"
        echo -e "${YELLOW}          journalctl -u mikudb -n 50${NC}"
    fi
else
    echo -e "${GRAY}[7/10] $(get_text 'SkipService')${NC}"
    echo -e "${GRAY}[8/10] $(get_text 'SkipEnable')${NC}"
    echo -e "${GRAY}[9/10] $(get_text 'SkipStart')${NC}"
fi

echo -e "${YELLOW}[10/10] $(get_text 'SettingPerms')${NC}"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
chmod 755 "$INSTALL_DIR/mikudb-server"
chmod 755 "$INSTALL_DIR/mikudb-cli"
echo -e "${GREEN}[OK] $(get_text 'PermsSet')${NC}"

echo ""
echo -e "${GREEN}=========================================="
echo -e "   $(get_text 'InstallComplete')"
echo -e "==========================================${NC}"
echo ""
echo -e "${CYAN}$(get_text 'InstallDir') $INSTALL_DIR${NC}"
echo -e "${CYAN}$(get_text 'DataDir')    $DATA_DIR${NC}"
echo -e "${CYAN}$(get_text 'ConfigDir')  $CONFIG_DIR/mikudb.toml${NC}"
echo ""

if [ "$SKIP_SERVICE" != "true" ]; then
    echo -e "${GREEN}$(get_text 'ServiceStatus')     $(get_text 'ServiceRunning')${NC}"
    echo -e "${CYAN}$(get_text 'ServiceName')       mikudb${NC}"
    echo ""
    echo -e "${YELLOW}$(get_text 'ManageService')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStart')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStop')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdRestart')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdStatus')${NC}"
    echo -e "${GRAY}  $(get_text 'CmdLogs')${NC}"
    echo ""
fi

echo -e "${YELLOW}$(get_text 'ConnectTo')${NC}"
echo -e "${GRAY}  $(get_text 'ConnectCmd')${NC}"
echo -e "${GRAY}  $(get_text 'Username')${NC}"
echo -e "${GRAY}  $(get_text 'Password')${NC}"
echo ""
echo -e "${RED}[IMPORTANT] $(get_text 'ChangePassword')${NC}"
echo -e "${YELLOW}$(get_text 'ChangeCmd')${NC}"
echo ""
echo -e "${YELLOW}$(get_text 'UninstallCmd')${NC}"
echo -e "${GRAY}  $(get_text 'UninstallHow')${NC}"
echo ""
