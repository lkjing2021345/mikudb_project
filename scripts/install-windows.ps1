# MikuDB Windows Installation Script
# PowerShell script for easy MikuDB deployment on Windows
# 支持中英文双语 / Supports Chinese and English

param(
    [string]$InstallPath = "C:\Program Files\MikuDB",
    [string]$DataPath = "C:\ProgramData\MikuDB",
    [switch]$SkipService = $false,
    [switch]$Uninstall = $false,
    [string]$Language = ""
)

$ErrorActionPreference = "Stop"
$Global:LANG = ""

# Language strings
$Strings = @{
    EN = @{
        Title = "MikuDB Windows Installation Script"
        SelectLang = "Please select language / 请选择语言:`n  1. English`n  2. 中文`n"
        LangPrompt = "Enter your choice (1-2)"
        InvalidLang = "Invalid choice, using English"
        NeedAdmin = "This script requires Administrator privileges"
        RunAsAdmin = "Please run PowerShell as Administrator and try again"
        Uninstalling = "Uninstalling MikuDB..."
        StopService = "Stopping MikuDB service..."
        ServiceStopped = "Service stopped and removed"
        ServiceNotFound = "Service not found"
        RemovingFiles = "Removing installation files..."
        FilesRemoved = "Installation directory removed"
        UpdatingPath = "Removing PATH environment variable..."
        PathUpdated = "PATH updated"
        DataPreserved = "Data directory preserved at:"
        ToRemoveData = "To remove data, manually delete:"
        UninstallSuccess = "MikuDB uninstalled successfully!"
        InstallPath = "Installation Path:"
        DataPath = "Data Path:"
        ExistingInstall = "Existing MikuDB installation detected!"
        ExistService = "Service:"
        ExistBinary = "Binary:"
        ExistConfig = "Config:"
        OverwritePrompt = "Do you want to overwrite the existing installation? (y/N)"
        Cancelled = "Installation aborted by user"
        RemovingExisting = "Removing existing installation..."
        ExistingRemoved = "Existing installation removed, continuing with fresh install..."
        CheckingPrereq = "Checking prerequisites..."
        RustNotFound = "Rust is not installed"
        InstallRust = "Please install Rust from: https://rustup.rs/"
        OrRunWinget = "Or run: winget install Rustlang.Rustup"
        FoundRust = "Found Rust:"
        CreatingDirs = "Creating directories..."
        DirsCreated = "Directories created"
        Building = "Building MikuDB..."
        BuildingServer = "Building mikudb-server..."
        BuildingCli = "Building mikudb-cli..."
        BuildComplete = "Build completed"
        BuildFailed = "Build failed:"
        InstallingBins = "Installing binaries..."
        BinsInstalled = "Binaries installed"
        CreatingConfig = "Creating default configuration..."
        ConfigCreated = "Configuration created:"
        AddingPath = "Adding to PATH..."
        AddedPath = "Added to system PATH"
        AlreadyInPath = "Already in PATH"
        CreatingService = "Creating Windows service..."
        ServiceCreated = "Service created: MikuDB"
        StartingService = "Starting service..."
        ServiceStarted = "Service started successfully"
        ServiceFailed = "Service failed to start, check logs at:"
        SkipService = "Skipping service installation"
        ManualStart = "Manual start"
        InstallComplete = "MikuDB Installation Complete!"
        ConfigFile = "Configuration File:"
        ServiceStatus = "Service Status:"
        ServiceRunning = "Running"
        ServiceName = "Service Name:"
        ManageService = "Manage service:"
        CmdStart = "Start:   Start-Service MikuDB"
        CmdStop = "Stop:    Stop-Service MikuDB"
        CmdRestart = "Restart: Restart-Service MikuDB"
        CmdStatus = "Status:  Get-Service MikuDB"
        ConnectTo = "Connect to MikuDB:"
        ConnectCmd = "mikudb-cli"
        Username = "Username: root"
        Password = "Password: mikudb_initial_password"
        ChangePassword = "Please change the default password!"
        ChangeCmd = "Use command: ALTER USER `"root`" PASSWORD `"your_secure_password`";"
        UninstallCmd = "Uninstall:"
        UninstallHow = ".\install-windows.ps1 -Uninstall"
    }
    CN = @{
        Title = "MikuDB Windows 安装脚本"
        SelectLang = "请选择语言 / Please select language:`n  1. English`n  2. 中文`n"
        LangPrompt = "输入你的选择 (1-2)"
        InvalidLang = "无效选择，使用中文"
        NeedAdmin = "此脚本需要管理员权限"
        RunAsAdmin = "请以管理员身份运行 PowerShell 后重试"
        Uninstalling = "正在卸载 MikuDB..."
        StopService = "停止 MikuDB 服务..."
        ServiceStopped = "服务已停止并移除"
        ServiceNotFound = "服务未找到"
        RemovingFiles = "删除安装文件..."
        FilesRemoved = "安装目录已删除"
        UpdatingPath = "移除 PATH 环境变量..."
        PathUpdated = "PATH 已更新"
        DataPreserved = "数据目录保留在:"
        ToRemoveData = "要删除数据，请手动删除:"
        UninstallSuccess = "MikuDB 卸载成功！"
        InstallPath = "安装路径:"
        DataPath = "数据路径:"
        ExistingInstall = "检测到已存在的 MikuDB 安装！"
        ExistService = "服务:"
        ExistBinary = "二进制:"
        ExistConfig = "配置:"
        OverwritePrompt = "是否覆盖现有安装？(y/N)"
        Cancelled = "用户取消安装"
        RemovingExisting = "移除现有安装..."
        ExistingRemoved = "现有安装已移除，继续全新安装..."
        CheckingPrereq = "检查先决条件..."
        RustNotFound = "Rust 未安装"
        InstallRust = "请从以下地址安装 Rust: https://rustup.rs/"
        OrRunWinget = "或运行: winget install Rustlang.Rustup"
        FoundRust = "找到 Rust:"
        CreatingDirs = "创建目录..."
        DirsCreated = "目录已创建"
        Building = "编译 MikuDB..."
        BuildingServer = "编译 mikudb-server..."
        BuildingCli = "编译 mikudb-cli..."
        BuildComplete = "编译完成"
        BuildFailed = "编译失败:"
        InstallingBins = "安装二进制文件..."
        BinsInstalled = "二进制文件已安装"
        CreatingConfig = "创建默认配置..."
        ConfigCreated = "配置已创建:"
        AddingPath = "添加到 PATH..."
        AddedPath = "已添加到系统 PATH"
        AlreadyInPath = "已在 PATH 中"
        CreatingService = "创建 Windows 服务..."
        ServiceCreated = "服务已创建: MikuDB"
        StartingService = "启动服务..."
        ServiceStarted = "服务启动成功"
        ServiceFailed = "服务启动失败，检查日志:"
        SkipService = "跳过服务安装"
        ManualStart = "手动启动"
        InstallComplete = "MikuDB 安装完成！"
        ConfigFile = "配置文件:"
        ServiceStatus = "服务状态:"
        ServiceRunning = "运行中"
        ServiceName = "服务名称:"
        ManageService = "管理服务:"
        CmdStart = "启动:   Start-Service MikuDB"
        CmdStop = "停止:   Stop-Service MikuDB"
        CmdRestart = "重启:   Restart-Service MikuDB"
        CmdStatus = "状态:   Get-Service MikuDB"
        ConnectTo = "连接到 MikuDB:"
        ConnectCmd = "mikudb-cli"
        Username = "用户名: root"
        Password = "密码: mikudb_initial_password"
        ChangePassword = "请修改默认密码！"
        ChangeCmd = "使用命令: ALTER USER `"root`" PASSWORD `"your_secure_password`";"
        UninstallCmd = "卸载:"
        UninstallHow = ".\install-windows.ps1 -Uninstall"
    }
}

function Get-Text {
    param([string]$Key)
    return $Strings[$Global:LANG][$Key]
}

function Select-Language {
    if ($Language -eq "en" -or $Language -eq "EN") {
        $Global:LANG = "EN"
        return
    }
    if ($Language -eq "zh" -or $Language -eq "cn" -or $Language -eq "CN") {
        $Global:LANG = "CN"
        return
    }

    Write-Host ""
    Write-Host $Strings.EN.SelectLang
    $choice = Read-Host $Strings.EN.LangPrompt

    switch ($choice) {
        "1" { $Global:LANG = "EN" }
        "2" { $Global:LANG = "CN" }
        default {
            Write-Host $Strings.EN.InvalidLang -ForegroundColor Yellow
            $Global:LANG = "CN"
        }
    }
}

Select-Language

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   $(Get-Text 'Title')" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

function Test-Administrator {
    $currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Administrator)) {
    Write-Host "[ERROR] $(Get-Text 'NeedAdmin')" -ForegroundColor Red
    Write-Host "$(Get-Text 'RunAsAdmin')" -ForegroundColor Yellow
    exit 1
}

function Uninstall-MikuDB {
    Write-Host "[1/4] $(Get-Text 'StopService')" -ForegroundColor Yellow
    try {
        Stop-Service -Name "MikuDB" -ErrorAction SilentlyContinue
        sc.exe delete MikuDB | Out-Null
        Write-Host "[OK] $(Get-Text 'ServiceStopped')" -ForegroundColor Green
    } catch {
        Write-Host "[SKIP] $(Get-Text 'ServiceNotFound')" -ForegroundColor Gray
    }

    Write-Host "[2/4] $(Get-Text 'RemovingFiles')" -ForegroundColor Yellow
    if (Test-Path $InstallPath) {
        Remove-Item -Path $InstallPath -Recurse -Force
        Write-Host "[OK] $(Get-Text 'FilesRemoved')" -ForegroundColor Green
    }

    Write-Host "[3/4] $(Get-Text 'UpdatingPath')" -ForegroundColor Yellow
    $path = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $newPath = ($path.Split(';') | Where-Object { $_ -ne $InstallPath }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
    Write-Host "[OK] $(Get-Text 'PathUpdated')" -ForegroundColor Green

    Write-Host "[4/4] $(Get-Text 'DataPreserved') $DataPath" -ForegroundColor Cyan
    Write-Host "$(Get-Text 'ToRemoveData') $DataPath" -ForegroundColor Cyan

    Write-Host ""
    Write-Host "[SUCCESS] $(Get-Text 'UninstallSuccess')" -ForegroundColor Green
    exit 0
}

if ($Uninstall) {
    Uninstall-MikuDB
}

function Test-ExistingInstallation {
    $hasService = $null -ne (Get-Service -Name "MikuDB" -ErrorAction SilentlyContinue)
    $hasBinary = Test-Path "$InstallPath\mikudb-server.exe"
    $hasConfig = Test-Path "$DataPath\config\mikudb.toml"

    if ($hasService -or $hasBinary -or $hasConfig) {
        Write-Host ""
        Write-Host "[WARNING] $(Get-Text 'ExistingInstall')" -ForegroundColor Yellow
        Write-Host ""

        if ($hasService) {
            $service = Get-Service -Name "MikuDB"
            Write-Host "  $(Get-Text 'ExistService') $($service.Status)" -ForegroundColor Cyan
        }
        if ($hasBinary) {
            Write-Host "  $(Get-Text 'ExistBinary') $InstallPath\mikudb-server.exe" -ForegroundColor Cyan
        }
        if ($hasConfig) {
            Write-Host "  $(Get-Text 'ExistConfig') $DataPath\config\mikudb.toml" -ForegroundColor Cyan
        }

        Write-Host ""
        $response = Read-Host "$(Get-Text 'OverwritePrompt')"

        if ($response -ne "y" -and $response -ne "Y") {
            Write-Host "[CANCELLED] $(Get-Text 'Cancelled')" -ForegroundColor Yellow
            exit 0
        }

        Write-Host ""
        Write-Host "[INFO] $(Get-Text 'RemovingExisting')" -ForegroundColor Yellow
        Uninstall-MikuDB
        Write-Host "[OK] $(Get-Text 'ExistingRemoved')" -ForegroundColor Green
        Write-Host ""
    }
}

Test-ExistingInstallation

Write-Host "[INFO] $(Get-Text 'InstallPath') $InstallPath" -ForegroundColor Cyan
Write-Host "[INFO] $(Get-Text 'DataPath') $DataPath" -ForegroundColor Cyan
Write-Host ""

Write-Host "[1/8] $(Get-Text 'CheckingPrereq')" -ForegroundColor Yellow

if (-not (Get-Command "rustc" -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] $(Get-Text 'RustNotFound')" -ForegroundColor Red
    Write-Host "$(Get-Text 'InstallRust')" -ForegroundColor Yellow
    Write-Host "$(Get-Text 'OrRunWinget')" -ForegroundColor Yellow
    exit 1
}

$rustVersion = rustc --version
Write-Host "[OK] $(Get-Text 'FoundRust') $rustVersion" -ForegroundColor Green

Write-Host "[2/8] $(Get-Text 'CreatingDirs')" -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null
New-Item -ItemType Directory -Force -Path "$DataPath\data" | Out-Null
New-Item -ItemType Directory -Force -Path "$DataPath\logs" | Out-Null
New-Item -ItemType Directory -Force -Path "$DataPath\config" | Out-Null
Write-Host "[OK] $(Get-Text 'DirsCreated')" -ForegroundColor Green

Write-Host "[3/8] $(Get-Text 'Building')" -ForegroundColor Yellow
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot

try {
    $env:CARGO_TERM_COLOR = "always"
    Write-Host "    $(Get-Text 'BuildingServer')" -ForegroundColor Gray
    cargo build --release -p mikudb-server 2>&1 | Out-Null

    Write-Host "    $(Get-Text 'BuildingCli')" -ForegroundColor Gray
    cargo build --release -p mikudb-cli 2>&1 | Out-Null

    Write-Host "[OK] $(Get-Text 'BuildComplete')" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] $(Get-Text 'BuildFailed') $_" -ForegroundColor Red
    Pop-Location
    exit 1
}

Write-Host "[4/8] $(Get-Text 'InstallingBins')" -ForegroundColor Yellow
Copy-Item -Path "target\release\mikudb-server.exe" -Destination "$InstallPath\mikudb-server.exe" -Force
Copy-Item -Path "target\release\mikudb-cli.exe" -Destination "$InstallPath\mikudb-cli.exe" -Force
Pop-Location
Write-Host "[OK] $(Get-Text 'BinsInstalled')" -ForegroundColor Green

Write-Host "[5/8] $(Get-Text 'CreatingConfig')" -ForegroundColor Yellow
$configContent = @"
# MikuDB Server Configuration

# Network settings
bind = "0.0.0.0"
port = 3939

# Data directory
data_dir = "$($DataPath -replace '\\', '\\')\data"

# Connection settings
max_connections = 1000
timeout_ms = 30000

# Storage settings
[storage]
page_size = 16384
cache_size = "1GB"
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
file = "$($DataPath -replace '\\', '\\')\logs\mikudb.log"
rotation = "daily"
max_files = 7
"@

$configPath = "$DataPath\config\mikudb.toml"
Set-Content -Path $configPath -Value $configContent -Encoding UTF8
Write-Host "[OK] $(Get-Text 'ConfigCreated') $configPath" -ForegroundColor Green

Write-Host "[6/8] $(Get-Text 'AddingPath')" -ForegroundColor Yellow
$path = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($path -notlike "*$InstallPath*") {
    $newPath = "$path;$InstallPath"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
    $env:Path = $newPath
    Write-Host "[OK] $(Get-Text 'AddedPath')" -ForegroundColor Green
} else {
    Write-Host "[SKIP] $(Get-Text 'AlreadyInPath')" -ForegroundColor Gray
}

if (-not $SkipService) {
    Write-Host "[7/8] $(Get-Text 'CreatingService')" -ForegroundColor Yellow

    $serviceParams = @{
        Name = "MikuDB"
        BinaryPathName = "`"$InstallPath\mikudb-server.exe`" --config `"$configPath`""
        DisplayName = "MikuDB Database Server"
        Description = "High-performance embedded database server"
        StartupType = "Automatic"
    }

    try {
        Stop-Service -Name "MikuDB" -ErrorAction SilentlyContinue
        sc.exe delete MikuDB | Out-Null
    } catch {}

    New-Service @serviceParams | Out-Null
    Write-Host "[OK] $(Get-Text 'ServiceCreated')" -ForegroundColor Green

    Write-Host "[8/8] $(Get-Text 'StartingService')" -ForegroundColor Yellow
    Start-Service -Name "MikuDB"

    Start-Sleep -Seconds 2
    $service = Get-Service -Name "MikuDB"
    if ($service.Status -eq "Running") {
        Write-Host "[OK] $(Get-Text 'ServiceStarted')" -ForegroundColor Green
    } else {
        Write-Host "[WARNING] $(Get-Text 'ServiceFailed')" -ForegroundColor Yellow
        Write-Host "         $DataPath\logs\mikudb.log" -ForegroundColor Yellow
    }
} else {
    Write-Host "[7/8] $(Get-Text 'SkipService')" -ForegroundColor Gray
    Write-Host "[8/8] $(Get-Text 'ManualStart')" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "   $(Get-Text 'InstallComplete')" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "$(Get-Text 'InstallPath') $InstallPath" -ForegroundColor Cyan
Write-Host "$(Get-Text 'DataPath')    $DataPath" -ForegroundColor Cyan
Write-Host "$(Get-Text 'ConfigFile')  $configPath" -ForegroundColor Cyan
Write-Host ""

if (-not $SkipService) {
    Write-Host "$(Get-Text 'ServiceStatus')     $(Get-Text 'ServiceRunning')" -ForegroundColor Green
    Write-Host "$(Get-Text 'ServiceName')       MikuDB" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "$(Get-Text 'ManageService')" -ForegroundColor Yellow
    Write-Host "  $(Get-Text 'CmdStart')" -ForegroundColor Gray
    Write-Host "  $(Get-Text 'CmdStop')" -ForegroundColor Gray
    Write-Host "  $(Get-Text 'CmdRestart')" -ForegroundColor Gray
    Write-Host "  $(Get-Text 'CmdStatus')" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "$(Get-Text 'ConnectTo')" -ForegroundColor Yellow
Write-Host "  $(Get-Text 'ConnectCmd')" -ForegroundColor Gray
Write-Host "  $(Get-Text 'Username')" -ForegroundColor Gray
Write-Host "  $(Get-Text 'Password')" -ForegroundColor Gray
Write-Host ""
Write-Host "[IMPORTANT] $(Get-Text 'ChangePassword')" -ForegroundColor Red
Write-Host "$(Get-Text 'ChangeCmd')" -ForegroundColor Yellow
Write-Host ""
Write-Host "$(Get-Text 'UninstallCmd')" -ForegroundColor Yellow
Write-Host "  $(Get-Text 'UninstallHow')" -ForegroundColor Gray
Write-Host ""
