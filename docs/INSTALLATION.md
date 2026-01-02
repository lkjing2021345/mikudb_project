# MikuDB 安装指南

本文档介绍如何在 Windows、Linux 和 OpenEuler 上快速安装部署 MikuDB。

## 目录

- [Windows 安装](#windows-安装)
- [Linux 通用安装](#linux-通用安装)
- [OpenEuler 优化安装](#openeuler-优化安装)
- [验证安装](#验证安装)
- [卸载](#卸载)
- [常见问题](#常见问题)

---

## Windows 安装

### 系统要求

- Windows 10/11 或 Windows Server 2016+
- 管理员权限
- 至少 2GB 可用内存
- 至少 10GB 可用磁盘空间

### 快速安装

1. **以管理员身份打开 PowerShell**

   ```powershell
   # 右键点击开始菜单 → Windows PowerShell (管理员)
   ```

2. **下载并运行安装脚本**

   ```powershell
   # 下载 MikuDB
   git clone https://gitlab.eduxiji.net/T202510423998020/project3035747-357273.git
   cd project3035747-357273

   # 执行安装脚本
   .\scripts\install-windows.ps1
   ```

3. **等待安装完成**

   脚本将自动完成以下步骤:
   - 检查 Rust 环境
   - 编译 MikuDB
   - 创建目录结构
   - 安装系统服务
   - 配置环境变量

### 自定义安装

```powershell
# 指定安装路径
.\scripts\install-windows.ps1 -InstallPath "D:\MikuDB" -DataPath "D:\MikuDB\Data"

# 跳过服务安装 (仅安装二进制文件)
.\scripts\install-windows.ps1 -SkipService
```

### 安装后配置

**默认配置文件位置**: `C:\ProgramData\MikuDB\config\mikudb.toml`

**修改配置**:
```powershell
notepad "C:\ProgramData\MikuDB\config\mikudb.toml"
```

**重启服务应用配置**:
```powershell
Restart-Service MikuDB
```

### 服务管理

```powershell
# 启动服务
Start-Service MikuDB

# 停止服务
Stop-Service MikuDB

# 查看服务状态
Get-Service MikuDB

# 查看服务日志
Get-Content "C:\ProgramData\MikuDB\logs\mikudb.log" -Tail 50 -Wait
```

---

## Linux 通用安装

### 支持的发行版

- Ubuntu 20.04+
- Debian 11+
- CentOS 7+
- Fedora 35+
- 其他主流 Linux 发行版

### 系统要求

- 至少 2GB 可用内存
- 至少 10GB 可用磁盘空间
- root 权限

### 快速安装

1. **下载 MikuDB**

   ```bash
   git clone https://gitlab.eduxiji.net/T202510423998020/project3035747-357273.git
   cd project3035747-357273
   ```

2. **执行安装脚本**

   ```bash
   sudo bash scripts/install-linux.sh
   ```

3. **等待安装完成**

   脚本将自动:
   - 检测操作系统
   - 安装依赖 (gcc, cmake 等)
   - 安装 Rust (如果未安装)
   - 编译 MikuDB
   - 创建系统用户和服务
   - 启动 MikuDB

### 自定义安装

```bash
# 指定安装路径
sudo INSTALL_DIR=/opt/mikudb DATA_DIR=/data/mikudb bash scripts/install-linux.sh

# 跳过服务安装
sudo SKIP_SERVICE=true bash scripts/install-linux.sh

# 指定服务用户
sudo SERVICE_USER=myuser bash scripts/install-linux.sh
```

### 安装后配置

**配置文件**: `/etc/mikudb/mikudb.toml`

**修改配置**:
```bash
sudo nano /etc/mikudb/mikudb.toml
```

**重启服务**:
```bash
sudo systemctl restart mikudb
```

### 服务管理

```bash
# 启动服务
sudo systemctl start mikudb

# 停止服务
sudo systemctl stop mikudb

# 重启服务
sudo systemctl restart mikudb

# 查看状态
sudo systemctl status mikudb

# 查看日志
sudo journalctl -u mikudb -f

# 开机自启
sudo systemctl enable mikudb

# 禁用开机自启
sudo systemctl disable mikudb
```

---

## OpenEuler 安装

### 快速安装

```bash
# 下载 MikuDB
git clone https://gitlab.eduxiji.net/T202510423998020/project3035747-357273.git
cd project3035747-357273

# 执行 OpenEuler 优化安装
sudo bash scripts/install-openeuler.sh
```

### 优化特性

安装脚本将自动启用以下优化:

1. **大页内存 (Huge Pages)**
   - 自动分配 2GB 大页内存
   - 减少 TLB miss,提升性能 10-20%

2. **NUMA 感知**
   - 自动检测 NUMA 拓扑
   - 绑定进程到 NUMA 节点 0
   - 优化内存访问延迟

3. **io_uring**
   - 启用高性能异步 I/O
   - 降低系统调用开销

4. **TCP 优化**
   - TCP_NODELAY: 禁用 Nagle 算法
   - TCP_CORK: 批量发送数据包
   - 优化网络参数

5. **编译优化**
   - 针对鲲鹏 CPU 优化 (`target-cpu=native`)
   - 启用 NEON 指令集
   - 使用 OpenEuler 特性标志

### 自定义优化

```bash
# 禁用大页内存
sudo ENABLE_HUGEPAGES=false bash scripts/install-openeuler.sh

# 禁用 NUMA 绑定
sudo ENABLE_NUMA=false bash scripts/install-openeuler.sh

# 组合配置
sudo ENABLE_HUGEPAGES=true ENABLE_NUMA=true bash scripts/install-openeuler.sh
```

### 性能验证

安装完成后,脚本会显示系统信息:

```
CPU Info:
    Architecture:        aarch64
    Model name:          Kunpeng-920
    CPU(s):              96

Huge Pages:
    HugePages_Total:    1024
    HugePages_Free:     1024
    Hugepagesize:       2048 kB

NUMA Topology:
    available: 4 nodes (0-3)
    node 0 cpus: 0-23
    node 0 size: 65536 MB
```

### OpenEuler 专项调优

**调整缓存大小** (根据内存容量):

```bash
sudo nano /etc/mikudb/mikudb.toml
```

```toml
[storage]
cache_size = "16GB"  # 调整为系统内存的 25-50%
```

**绑定到特定 CPU 核心**:

```toml
[openeuler]
cpu_affinity = [0, 1, 2, 3]  # 绑定到 CPU 0-3
```

---

## 验证安装

### 1. 检查服务状态

**Windows**:
```powershell
Get-Service MikuDB
```

**Linux / OpenEuler**:
```bash
sudo systemctl status mikudb
```

### 2. 连接数据库

```bash
mikudb-cli
```

输入默认凭据:
- **用户名**: `root`
- **密码**: `mikudb_initial_password`

### 3. 执行测试查询

```sql
-- 创建测试集合
CREATE COLLECTION test_collection;

-- 插入测试数据
INSERT INTO test_collection {
    "name": "MikuDB",
    "version": "0.1.2",
    "status": "running"
};

-- 查询数据
FIND test_collection WHERE name = "MikuDB";

-- 删除测试集合
DROP COLLECTION test_collection;
```

### 4. 修改默认密码

```sql
ALTER USER "root" PASSWORD "your_secure_password_here";
```

退出并使用新密码重新登录:
```bash
exit
mikudb-cli
```

---

## 卸载

### Windows

```powershell
# 以管理员身份运行
.\scripts\install-windows.ps1 -Uninstall
```

数据目录保留在 `C:\ProgramData\MikuDB`,如需删除:
```powershell
Remove-Item -Path "C:\ProgramData\MikuDB" -Recurse -Force
```

### Linux / OpenEuler

```bash
sudo UNINSTALL=true bash scripts/install-linux.sh
# 或
sudo UNINSTALL=true bash scripts/install-openeuler.sh
```

删除数据目录:
```bash
sudo rm -rf /var/lib/mikudb
```

---

## 常见问题

### Q1: 安装时提示 "Rust not found"

**Windows**:
```powershell
winget install Rustlang.Rustup
# 或访问 https://rustup.rs/
```

**Linux**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Q2: 编译失败 "zstd-sys build error" (Windows)

这是因为 Windows 缺少 C 编译器。

**解决方法**:
1. 安装 Visual Studio 2022 Build Tools
2. 或安装 MSYS2/MinGW-w64
3. 确保 PATH 中包含编译器路径

推荐使用 Linux 或 OpenEuler 进行生产部署。

### Q3: 服务启动失败

**查看日志**:

Windows:
```powershell
Get-Content "C:\ProgramData\MikuDB\logs\mikudb.log" -Tail 100
```

Linux/OpenEuler:
```bash
sudo journalctl -u mikudb -n 100
```

**常见原因**:
- 端口 3939 被占用
- 数据目录权限不足
- 配置文件格式错误

### Q4: 无法连接到 MikuDB

**检查服务是否运行**:
```bash
# Linux
sudo systemctl status mikudb

# Windows
Get-Service MikuDB
```

**检查端口是否监听**:
```bash
# Linux
sudo netstat -tlnp | grep 3939

# Windows
netstat -an | findstr 3939
```

**检查防火墙**:
```bash
# Linux (CentOS/OpenEuler)
sudo firewall-cmd --add-port=3939/tcp --permanent
sudo firewall-cmd --reload

# Linux (Ubuntu)
sudo ufw allow 3939/tcp
```

### Q5: OpenEuler 大页内存未启用

**检查大页状态**:
```bash
grep HugePages /proc/meminfo
```

**手动启用**:
```bash
sudo sysctl -w vm.nr_hugepages=1024
echo "vm.nr_hugepages=1024" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### Q6: 如何迁移数据

**备份**:
```bash
# 停止服务
sudo systemctl stop mikudb

# 备份数据目录
sudo tar -czf mikudb-backup-$(date +%Y%m%d).tar.gz /var/lib/mikudb/data

# 启动服务
sudo systemctl start mikudb
```

**恢复**:
```bash
# 停止服务
sudo systemctl stop mikudb

# 恢复数据
sudo tar -xzf mikudb-backup-20240101.tar.gz -C /

# 修复权限
sudo chown -R mikudb:mikudb /var/lib/mikudb

# 启动服务
sudo systemctl start mikudb
```

### Q7: 集群部署

单机安装脚本仅适用于单节点部署。

集群部署请参考:
- [CLUSTER-DEPLOYMENT.md](./CLUSTER-DEPLOYMENT.md) - 集群部署指南
- [examples/cluster-node.toml](../examples/cluster-node.toml) - 集群配置示例

---

## 获取帮助

- **文档**: [docs/](../docs/)
- **示例**: [examples/](../examples/)
- **问题反馈**: [GitHub Issues](https://github.com/yourusername/mikudb/issues)

---

**版本**: v0.1.2
**更新日期**: 2024-01-03
**许可证**: GNU General Public License v3.0
