<br>

<div align="center">
    <img src="https://vip.123pan.cn/1842292950/ymjew503t0m000d7w32xzgmn6upslg2jDIYxDqi2DqryDcxwDwizAY==.svg" width=100/>
</div>
<div align="center">
    <h1>MikuDB</h1>
    <h2>基于 OpenEuler 操作系统的非关系型数据库应用</h2>
</div>
<br>
<div align="center">
    <img src="https://vip.123pan.cn/1842292950/yk6baz03t0l000d7w33fy0bj9oqjoo4zDIYxDqi2DqryDcxwDwizAY==.svg"/>
    <img src="https://img.shields.io/badge/Language-Rust-red">
    <img src="https://img.shields.io/badge/License-MIT-Black">
</div>
<br>

---

## 项目概述

MikuDB 是一款专为 OpenEuler 操作系统设计的高性能非关系型文档数据库，采用自研的 BOML（Binary Object Markup Language）格式存储数据，目标是超越 MongoDB 的稳定性和性能表现。

### 核心特性

- 🚀 **高性能**：基于 Rust 语言开发，零成本抽象，内存安全
- 📦 **BOML 格式**：自研二进制文档格式，对标 BSON，更高效的序列化/反序列化
- 🐧 **OpenEuler 原生**：深度适配 OpenEuler 系统特性
- 🤖 **AI 原生集成**：内置 MCP Server，支持 OpenAI 格式 API 接入
- 💻 **智能 CLI**：MikuDB-CLI 带语法提示、Tab 补全、错误纠正
- 🔐 **安全可靠**：完善的认证授权机制

### 默认配置

| 配置项 | 默认值 |
|--------|--------|
| 端口 | 3939 |
| 默认用户名 | miku |
| 默认密码 | mikumiku3939 |

---

## 开发方案

### 一、项目架构

```
mikudb/
├── mikudb-core/          # 核心引擎库
│   ├── boml/             # BOML 格式解析器
│   ├── storage/          # 存储引擎
│   ├── index/            # 索引系统
│   ├── query/            # 查询引擎
│   └── transaction/      # 事务管理
├── mikudb-server/        # 数据库服务器
│   ├── network/          # 网络层（TCP/Unix Socket）
│   ├── protocol/         # 通信协议
│   ├── auth/             # 认证授权
│   └── cluster/          # 集群管理
├── mikudb-cli/           # 命令行客户端
│   ├── repl/             # 交互式环境
│   ├── completer/        # 自动补全
│   ├── highlighter/      # 语法高亮
│   └── validator/        # 语法校验
├── mikudb-mcp/           # MCP Server 模块
│   ├── server/           # MCP 服务实现
│   ├── tools/            # MCP 工具定义
│   └── resources/        # MCP 资源定义
├── mikudb-ai/            # AI 集成模块
│   ├── openai/           # OpenAI 格式适配
│   ├── embedding/        # 向量嵌入
│   └── nl2mql/           # 自然语言转查询
└── mikudb-openeuler/     # OpenEuler 适配层
    ├── systemd/          # 服务管理
    ├── selinux/          # SELinux 策略
    └── tuning/           # 性能调优
```

### 二、技术选型

| 模块 | 技术方案 |
|------|----------|
| 开发语言 | Rust (MSRV 1.75+) |
| 异步运行时 | Tokio |
| 网络框架 | Tower + Hyper |
| 序列化 | 自研 BOML + Serde |
| 存储引擎 | 自研 LSM-Tree / B+Tree 混合引擎 |
| CLI 框架 | Rustyline + Clap |
| 配置管理 | TOML |
| 日志系统 | Tracing |
| 测试框架 | 内置测试 + Criterion (基准测试) |

---

## 项目进度

### 当前状态：v0.1.0 开发中

#### 已完成模块

| 模块 | 状态 | 说明 |
|------|------|------|
| mikudb-common | ✅ 完成 | 公共类型、错误处理、平台检测 |
| mikudb-boml | ✅ 完成 | BOML 二进制格式序列化/反序列化 |
| mikudb-storage | ✅ 完成 | 基于 RocksDB 的存储引擎 |
| mikudb-query | ✅ 完成 | MQL 词法分析、解析器、执行器 |
| mikudb-core | ✅ 完成 | 核心引擎整合层 |
| mikudb-server | ✅ 完成 | 数据库服务器 (MikuWire 协议、认证、会话管理) |
| mikudb-cli | ✅ 完成 | 命令行客户端 (REPL、语法高亮、自动补全) |

#### 已实现功能

**mikudb-boml (BOML 格式库)**
- [x] 完整数据类型支持 (Null, Boolean, Int32/64/128, Float32/64, Decimal, String, Binary, DateTime, ObjectId, UUID, Array, Document)
- [x] 高效序列化器 (encode_to_vec)
- [x] 反序列化器 (decode)
- [x] Serde 集成
- [x] Document API (insert, get, remove, iter)
- [x] 基准测试 (Criterion)

**mikudb-storage (存储引擎)**
- [x] RocksDB 集成
- [x] Collection CRUD 操作
- [x] WAL 日志模块
- [x] LRU 缓存
- [x] 压缩支持 (LZ4/Zstd)
- [x] 后台压缩任务

**mikudb-query (查询引擎)**
- [x] MQL 词法分析器 (Logos)
- [x] MQL 解析器
- [x] AST 定义
- [x] 查询计划器
- [x] 查询执行器
- [x] 过滤器引擎
- [x] 索引管理

**mikudb-core (核心引擎)**
- [x] Database 管理
- [x] Transaction 事务支持
- [x] Session 会话管理
- [x] Client 异步客户端
- [x] ConnectionString 解析
- [x] Cursor 游标迭代器
- [x] Pipeline 聚合管道构建器
- [x] Builder 模式配置

**mikudb-common (公共库)**
- [x] ObjectId 生成
- [x] 错误类型定义
- [x] 平台检测 (OpenEuler/Linux/Windows/macOS)
- [x] 配置管理

**mikudb-server (数据库服务器)**
- [x] MikuWire 二进制协议
  - [x] MAGIC 魔数校验 ("MIKU")
  - [x] 消息头 (版本、操作码、请求ID、标志位、长度)
  - [x] 20+ 操作码 (CRUD、索引、事务等)
- [x] 网络层
  - [x] TCP 监听器 (异步 Tokio)
  - [x] Unix Socket 支持 (Linux)
  - [x] 连接限流 (Semaphore)
- [x] 认证模块
  - [x] 用户名/密码验证
  - [x] 会话管理 (创建、超时、清理)
- [x] 请求处理器
  - [x] Query/Insert/Update/Delete/Find
  - [x] CreateCollection/DropCollection/ListCollections
  - [x] CreateDatabase/DropDatabase/ListDatabases
  - [x] Ping/Pong 心跳
- [x] OpenEuler 优化 (条件编译)
  - [x] 大页内存 (Huge Pages)
  - [x] NUMA 感知内存分配
  - [x] io_uring 异步 I/O
  - [x] CPU 亲和性绑定
  - [x] TCP 内核参数调优

**mikudb-cli (命令行客户端)**
- [x] 异步 TCP 客户端
  - [x] MikuWire 协议实现
  - [x] 自动重连机制
  - [x] 超时处理
- [x] REPL 交互环境 (Rustyline)
  - [x] 多行输入支持
  - [x] 历史记录持久化 (~/.mikudb_history)
  - [x] Emacs 快捷键
- [x] 语法高亮 (MqlHighlighter)
  - [x] 关键字着色 (青色)
  - [x] 字符串着色 (绿色)
  - [x] 数字着色 (黄色)
  - [x] 操作符着色 (品红)
- [x] 自动补全 (MqlCompleter)
  - [x] MQL 关键字补全
  - [x] 上下文感知补全
- [x] 输出格式化
  - [x] JSON 美化输出
  - [x] 表格输出 (tabled)
- [x] 内置命令
  - [x] .help / .exit / .quit
  - [x] .status / .clear

#### 编译状态

| 平台 | 状态 | 备注 |
|------|------|------|
| Windows (MSVC) | ✅ 通过 | 需设置 BINDGEN_EXTRA_CLANG_ARGS |
| Linux (Ubuntu) | ✅ 通过 | - |
| Linux (OpenEuler) | ✅ 通过 | 支持 --features openeuler |

---

## 功能列表

### 第一阶段：核心引擎（v0.1.0）

#### 1. BOML 格式解析器
- [x] 定义 BOML 规范文档
- [x] 实现基础数据类型
  - [x] Null、Boolean、Integer (i32/i64/i128)
  - [x] Float (f32/f64)、Decimal (高精度)
  - [x] String (UTF-8)、Binary (字节数组)
  - [x] DateTime、Timestamp、Date、Time
  - [x] ObjectId (12字节唯一标识)
  - [x] UUID
  - [x] Array、Document (嵌套文档)
  - [ ] Regex、JavaScript (可选)
- [x] 实现序列化器 (Rust → BOML)
- [x] 实现反序列化器 (BOML → Rust)
- [x] Serde 集成支持
- [ ] 与 BSON/JSON 互转工具
- [x] 基准测试（对比 BSON 性能）

#### 2. 存储引擎
- [x] 页面管理器 (Page Manager)
  - [x] 4KB/8KB/16KB 可配置页面大小
  - [x] 页面缓存 (LRU/CLOCK)
  - [x] 脏页刷写策略
- [x] WAL (Write-Ahead Logging)
  - [x] 日志记录格式
  - [x] 日志刷写策略
  - [ ] 崩溃恢复机制
- [x] 文档存储
  - [x] 变长记录存储
  - [x] 空闲空间管理
  - [x] 文档压缩 (LZ4/Zstd)
- [x] 索引引擎
  - [x] B+Tree 索引 (RocksDB)
  - [ ] 哈希索引
  - [ ] 复合索引
  - [ ] 唯一索引
  - [ ] 稀疏索引
  - [ ] TTL 索引
  - [ ] 全文索引 (基础)
  - [ ] 地理空间索引 (2dsphere)
- [x] 事务管理
  - [x] MVCC (多版本并发控制)
  - [x] 读已提交隔离级别
  - [x] 可重复读隔离级别
  - [x] 快照隔离
  - [ ] 分布式事务 (2PC)

#### 3. 查询引擎
- [x] 查询解析器
- [x] 查询优化器
  - [x] 基于规则的优化 (RBO)
  - [ ] 基于代价的优化 (CBO)
  - [x] 索引选择
- [x] 执行器
  - [x] 迭代器模型
  - [ ] 向量化执行 (可选)
- [x] 聚合管道
  - [x] $match, $project, $group
  - [x] $sort, $limit, $skip
  - [ ] $lookup (关联查询)
  - [ ] $unwind, $bucket

---

### 第二阶段：服务器与协议（v0.2.0）

#### 4. 数据库服务器
- [x] 网络层
  - [x] TCP 监听器 (端口 3939)
  - [x] Unix Domain Socket 支持 (Linux)
  - [ ] TLS/SSL 加密
  - [x] 连接池管理 (Semaphore 限流)
  - [x] 请求限流
- [x] 通信协议
  - [x] 自定义二进制协议 (MikuWire)
  - [x] 消息帧格式定义 (MAGIC + Header + Payload)
  - [x] 请求/响应模型
  - [x] 心跳机制 (Ping/Pong)
- [x] 认证授权
  - [x] 用户认证 (用户名/密码)
  - [ ] 角色管理
  - [ ] SCRAM-SHA-256 认证
  - [ ] 基于角色的访问控制 (RBAC)
  - [ ] 数据库/集合级权限
- [x] 会话管理
  - [x] 会话创建/销毁
  - [x] 会话超时
  - [x] 游标管理
- [x] OpenEuler 优化 (Linux)
  - [x] 大页内存支持
  - [x] NUMA 感知
  - [x] io_uring 异步 I/O
  - [x] CPU 亲和性
  - [x] TCP 调优

#### 5. MQL 查询语言设计
```
// MQL (Miku Query Language) 语法设计

// 数据库操作
USE database_name
SHOW DATABASE
CREATE DATABASE db_name
DROP DATABASE db_name

// 集合操作
SHOW COLLECTION
CREATE COLLECTION collection_name
DROP COLLECTION collection_name

// 文档 CRUD
INSERT INTO collection_name {field1: value1, field2: value2}
INSERT INTO collection_name [{doc1}, {doc2}, {doc3}]

FIND collection_name                           // 查询所有
FIND collection_name WHERE field = value       // 条件查询
FIND collection_name WHERE field > 10 AND field2 = "test"
FIND collection_name WHERE field IN [1, 2, 3]
FIND collection_name WHERE field LIKE "pattern%"
FIND collection_name WHERE nested.field = value
FIND collection_name SELECT field1, field2     // 投影
FIND collection_name ORDER BY field ASC|DESC
FIND collection_name LIMIT 10 SKIP 20

UPDATE collection_name SET field = value WHERE condition
UPDATE collection_name SET field += 1 WHERE condition        // 增量更新
UPDATE collection_name UNSET field WHERE condition           // 删除字段
UPDATE collection_name PUSH array_field = value WHERE cond   // 数组追加

DELETE FROM collection_name WHERE condition
DELETE FROM collection_name                    // 清空集合

// 聚合查询
AGGREGATE collection_name
  | MATCH field > 10
  | GROUP BY field1 AS {count: COUNT(), sum: SUM(field2)}
  | SORT count DESC
  | LIMIT 10

// 索引操作
CREATE INDEX idx_name ON collection_name (field1 ASC, field2 DESC)
CREATE UNIQUE INDEX idx_name ON collection_name (field)
CREATE TEXT INDEX idx_name ON collection_name (field)
DROP INDEX idx_name ON collection_name
SHOW INDEX ON collection_name

// 事务
BEGIN TRANSACTION
  INSERT INTO ...
  UPDATE ...
COMMIT

// AI 集成语法
AI QUERY "用自然语言描述你想要的查询"
AI ANALYZE collection_name    // AI 分析集合结构
AI SUGGEST INDEX collection_name  // AI 建议索引

// 管理命令
SHOW STATUS
SHOW USERS
CREATE USER username WITH PASSWORD 'password' ROLE role_name
DROP USER username
GRANT role ON database.collection TO username
REVOKE role ON database.collection FROM username
```

---

### 第三阶段：CLI 客户端（v0.3.0）

#### 6. MikuDB-CLI
- [x] REPL 交互环境
  - [x] 多行输入支持
  - [x] 历史记录 (持久化)
  - [x] 快捷键绑定 (Emacs 模式)
- [x] 语法高亮
  - [x] 关键字高亮
  - [x] 字符串/数字高亮
  - [x] 错误标红
- [x] 自动补全
  - [x] 关键字补全
  - [x] 数据库/集合名补全
  - [ ] 字段名补全 (基于 schema 推断)
  - [x] 智能上下文补全
- [ ] 语法校验
  - [ ] 实时语法检查
  - [ ] 错误提示与纠正建议
  - [ ] Did you mean "xxx"? 提示
- [x] 输出格式化
  - [x] JSON 美化输出
  - [x] 表格输出
  - [ ] CSV 导出
- [x] 脚本模式
  - [x] 文件执行 `mikudb-cli < script.mql`
  - [x] 管道支持
- [x] 连接管理
  - [x] 连接字符串解析
  - [ ] 多服务器切换
  - [x] 连接配置文件

---

### 第四阶段：AI 与 MCP 集成（v0.4.0）

#### 7. MCP Server
- [ ] MCP 协议实现
  - [ ] stdio 传输
  - [ ] SSE 传输
- [ ] 工具定义
  - [ ] `query` - 执行 MQL 查询
  - [ ] `insert` - 插入文档
  - [ ] `update` - 更新文档
  - [ ] `delete` - 删除文档
  - [ ] `aggregate` - 聚合查询
  - [ ] `schema` - 获取集合结构
  - [ ] `stats` - 获取统计信息
- [ ] 资源定义
  - [ ] `databases` - 数据库列表
  - [ ] `collections` - 集合列表
  - [ ] `documents` - 文档资源
- [ ] Prompts 定义
  - [ ] 查询辅助提示
  - [ ] Schema 设计建议

#### 8. AI 集成模块
- [ ] OpenAI 格式适配器
  - [ ] Chat Completions API
  - [ ] Embeddings API
  - [ ] 支持自定义 base_url
- [ ] 多模型支持
  - [ ] OpenAI (GPT-4, GPT-3.5)
  - [ ] Anthropic Claude
  - [ ] 本地模型 (Ollama)
  - [ ] Azure OpenAI
- [ ] 向量搜索
  - [ ] 向量字段类型
  - [ ] 向量索引 (HNSW)
  - [ ] 相似度搜索 API
- [ ] 自然语言查询
  - [ ] NL → MQL 转换
  - [ ] 查询意图识别
  - [ ] 上下文对话支持
- [ ] 智能功能
  - [ ] 自动索引建议
  - [ ] 查询优化建议
  - [ ] Schema 设计助手
  - [ ] 数据异常检测

---

### 第五阶段：OpenEuler 适配与生产就绪（v0.5.0）

#### 9. OpenEuler 深度适配
- [ ] 系统服务
  - [ ] systemd 服务文件
  - [ ] 自动启动配置
  - [ ] 日志集成 (journald)
- [ ] 安全加固
  - [ ] SELinux 策略模块
  - [ ] Seccomp 过滤器
  - [ ] Capabilities 限制
- [ ] 性能调优
  - [ ] 内核参数优化脚本
  - [ ] 大页内存支持
  - [ ] NUMA 感知
  - [ ] io_uring 异步 I/O
- [ ] 包管理
  - [ ] RPM 打包规范
  - [ ] DNF 仓库配置
- [ ] 监控集成
  - [ ] Prometheus metrics 导出
  - [ ] Grafana 仪表盘模板

#### 10. 集群与高可用（v0.6.0+）
- [ ] 副本集
  - [ ] 主从复制
  - [ ] 自动故障转移
  - [ ] 读写分离
- [ ] 分片集群
  - [ ] 范围分片
  - [ ] 哈希分片
  - [ ] 分片键选择
  - [ ] 数据均衡
- [ ] 管理工具
  - [ ] 集群状态监控
  - [ ] 在线扩缩容
  - [ ] 数据迁移工具

---

## 配置文件设计

### 服务器配置 (mikudb.toml)

```toml
[server]
bind = "0.0.0.0"
port = 3939
unix_socket = "/var/run/mikudb/mikudb.sock"
max_connections = 10000
timeout = 30000  # ms

[storage]
data_dir = "/var/lib/mikudb/data"
wal_dir = "/var/lib/mikudb/wal"
page_size = 16384  # 16KB
cache_size = "1GB"
compression = "lz4"  # none, lz4, zstd

[auth]
enabled = true
default_user = "miku"
default_password = "mikumiku3939"

[security]
tls_enabled = false
tls_cert = "/etc/mikudb/ssl/cert.pem"
tls_key = "/etc/mikudb/ssl/key.pem"

[log]
level = "info"  # trace, debug, info, warn, error
file = "/var/log/mikudb/mikudb.log"
rotation = "daily"
max_files = 7

[ai]
enabled = false
provider = "openai"  # openai, anthropic, ollama, azure

[ai.openai]
api_key = ""
base_url = "https://api.openai.com/v1"
model = "gpt-4"
embedding_model = "text-embedding-3-small"

[ai.anthropic]
api_key = ""
model = "claude-3-opus-20240229"

[ai.ollama]
base_url = "http://localhost:11434"
model = "llama2"

[mcp]
enabled = true
transport = "stdio"  # stdio, sse

[replication]
enabled = false
role = "primary"  # primary, secondary
replica_set = "rs0"

[metrics]
enabled = true
prometheus_port = 9939
```

---

## 开发路线图

| 版本 | 里程碑 | 预计时间 |
|------|--------|----------|
| v0.1.0 | BOML 格式 + 存储引擎核心 | 8 周 |
| v0.2.0 | 服务器 + MQL 查询语言 | 6 周 |
| v0.3.0 | MikuDB-CLI 完整功能 | 4 周 |
| v0.4.0 | MCP Server + AI 集成 | 4 周 |
| v0.5.0 | OpenEuler 适配 + 生产就绪 | 4 周 |
| v0.6.0 | 副本集 + 分片集群 | 8 周 |
| v1.0.0 | 正式发布 | - |

---

## 性能目标

| 指标 | 目标值 | 对比 MongoDB |
|------|--------|--------------|
| 单文档插入 QPS | > 100,000 | +30% |
| 批量插入吞吐 | > 500,000 docs/s | +50% |
| 简单查询延迟 (P99) | < 1ms | -40% |
| 复杂聚合查询 | 同等复杂度 -30% | -30% |
| 内存占用 | 基准 -20% | -20% |
| 冷启动时间 | < 2s | -50% |

---

## 目录结构（计划）

```
mikudb/
├── Cargo.toml                 # 工作空间配置
├── Cargo.lock
├── README.md
├── LICENSE
├── docs/                      # 文档
│   ├── BOML-spec.md          # BOML 格式规范
│   ├── MQL-spec.md           # MQL 语法规范
│   ├── protocol.md           # 通信协议文档
│   └── api/                  # API 文档
├── crates/
│   ├── mikudb-boml/          # BOML 格式库
│   ├── mikudb-storage/       # 存储引擎
│   ├── mikudb-query/         # 查询引擎
│   ├── mikudb-server/        # 服务器
│   ├── mikudb-cli/           # CLI 客户端
│   ├── mikudb-mcp/           # MCP Server
│   ├── mikudb-ai/            # AI 模块
│   └── mikudb-common/        # 公共工具库
├── config/
│   └── mikudb.example.toml   # 示例配置
├── scripts/
│   ├── install.sh            # 安装脚本
│   └── openeuler/            # OpenEuler 专用脚本
├── systemd/
│   └── mikudb.service        # systemd 服务文件
├── tests/
│   ├── integration/          # 集成测试
│   └── benchmark/            # 性能基准测试
└── tools/
    ├── boml2json/            # BOML 转 JSON 工具
    └── mongoimport/          # MongoDB 数据导入工具
```

---

## 编译指南

### 系统要求

- Rust 1.75+ (推荐使用 rustup 安装)
- Clang/LLVM (用于编译 RocksDB 和 zstd)
- CMake 3.16+

### Windows 编译

#### 1. 安装依赖

```powershell
# 安装 Rust
winget install Rustlang.Rustup

# 安装 Visual Studio Build Tools (包含 MSVC)
winget install Microsoft.VisualStudio.2022.BuildTools

# 安装 LLVM/Clang (必需，用于编译 zstd-sys 和 rocksdb)
winget install LLVM.LLVM

# 设置环境变量 (PowerShell)
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
# 永久设置 (以管理员身份运行)
[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "Machine")
```

#### 2. 编译项目

```powershell
# 克隆项目
git clone https://github.com/your-repo/mikudb.git
cd mikudb

# 编译 (Debug 模式)
cargo build

# 编译 (Release 模式，推荐生产环境)
cargo build --release

# 仅检查代码 (不生成二进制)
cargo check

# 运行测试
cargo test

# 仅编译 CLI 客户端
cargo build --release -p mikudb-cli

# 仅编译服务器
cargo build --release -p mikudb-server

# 直接运行 CLI
cargo run -p mikudb-cli -- --help

# 编译后的二进制文件位置
# Debug: target\debug\mikudb-cli.exe, target\debug\mikudb-server.exe
# Release: target\release\mikudb-cli.exe, target\release\mikudb-server.exe
```

#### 3. 常见问题

**Q: 报错 "Unable to find libclang"**
```
A: 确保已安装 LLVM 并设置 LIBCLANG_PATH 环境变量
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

**Q: 链接错误 "LINK : fatal error LNK1181"**
```
A: 确保已安装 Visual Studio Build Tools 及 C++ 开发工具
```

---

### Linux 编译 (Ubuntu/Debian)

#### 1. 安装依赖

```bash
# 更新包管理器
sudo apt update

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 安装编译依赖
sudo apt install -y build-essential cmake clang libclang-dev pkg-config libssl-dev
```

#### 2. 编译项目

```bash
# 克隆项目
git clone https://github.com/your-repo/mikudb.git
cd mikudb

# 编译
cargo build --release

# 运行测试
cargo test

# 仅编译 CLI 客户端
cargo build --release -p mikudb-cli

# 仅编译服务器
cargo build --release -p mikudb-server

# 安装到系统 (可选)
cargo install --path crates/mikudb-server
cargo install --path crates/mikudb-cli

# 或手动复制二进制文件
sudo cp target/release/mikudb-server /usr/local/bin/
sudo cp target/release/mikudb-cli /usr/local/bin/

# 或使用安装脚本 (推荐)
sudo bash scripts/install.sh
# 安装脚本会询问是否安装 CLI,默认为 Yes
```

---

### Linux 编译 (OpenEuler/RHEL/CentOS)

#### 1. 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 安装编译依赖
sudo dnf install -y gcc gcc-c++ cmake clang clang-devel openssl-devel pkg-config

# OpenEuler 特定优化 (可选)
sudo dnf install -y numactl-devel liburing-devel
```

#### 2. 编译项目

```bash
# 编译 (启用 OpenEuler 优化)
cargo build --release --features openeuler

# 不带优化特性编译
cargo build --release

# 仅编译 CLI 客户端
cargo build --release -p mikudb-cli

# 仅编译服务器
cargo build --release -p mikudb-server
```

#### 3. 安装到系统 (OpenEuler)

```bash
# 方式 1: 使用安装脚本 (推荐，包含完整的 systemd 服务配置)
sudo bash scripts/openeuler/install.sh
# 脚本会自动询问是否安装 mikudb-cli,默认为 Yes

# 方式 2: 使用 cargo install
cargo install --path crates/mikudb-server
cargo install --path crates/mikudb-cli

# 方式 3: 手动复制二进制文件
sudo cp target/release/mikudb-server /usr/local/bin/
sudo cp target/release/mikudb-cli /usr/local/bin/

# 如果使用方式 2 或 3,还需要手动创建目录和配置
# 创建数据目录
sudo mkdir -p /var/lib/mikudb/data
sudo mkdir -p /var/log/mikudb

# 复制 systemd 服务文件
sudo cp systemd/mikudb.service /etc/systemd/system/

# 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable mikudb
sudo systemctl start mikudb

# 查看状态
sudo systemctl status mikudb

# 验证 CLI 安装
mikudb-cli --version
```

---

### macOS 编译

#### 1. 安装依赖

```bash
# 安装 Homebrew (如果没有)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 安装 Rust
brew install rustup-init
rustup-init

# 安装编译依赖
brew install cmake llvm

# 设置环境变量
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
```

#### 2. 编译项目

```bash
cargo build --release

# 仅编译 CLI 客户端
cargo build --release -p mikudb-cli

# 仅编译服务器
cargo build --release -p mikudb-server
```

#### 3. 安装到系统 (可选)

```bash
# 方式 1: 使用 cargo install (推荐)
cargo install --path crates/mikudb-server
cargo install --path crates/mikudb-cli

# 方式 2: 手动复制二进制文件
sudo cp target/release/mikudb-server /usr/local/bin/
sudo cp target/release/mikudb-cli /usr/local/bin/

# 验证安装
mikudb-server --version
mikudb-cli --version
```

---

### Docker 编译

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

RUN apt-get update && apt-get install -y \
    cmake clang libclang-dev pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mikudb-server /usr/local/bin/
COPY --from=builder /app/target/release/mikudb-cli /usr/local/bin/
EXPOSE 3939
CMD ["mikudb-server"]
```

```bash
# 构建镜像
docker build -t mikudb:latest .

# 运行容器
docker run -d -p 3939:3939 -v mikudb-data:/var/lib/mikudb mikudb:latest
```

---

### 编译选项

| Feature | 说明 | 命令 |
|---------|------|------|
| `default` | 默认特性 | `cargo build` |
| `openeuler` | OpenEuler 优化 (io_uring) | `cargo build --features openeuler` |
| `hugepages` | 大页内存支持 | `cargo build --features hugepages` |
| `full` | 所有特性 | `cargo build --all-features` |[    

### 验证安装

```bash
# 检查版本
mikudb-server --version
mikudb-cli --version

# 启动服务器
mikudb-server

# 使用 CLI 连接
mikudb-cli --host localhost --port 3939 --user miku --password mikumiku3939
```

---

## 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

---

## 许可证

本项目采用 GNU 许可证 - 详见 [LICENSE](LICENSE) 文件
