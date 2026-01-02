//! CLI 非交互模式模块
//!
//! 本模块实现命令行非交互式执行:
//! - 执行单条 MQL 查询
//! - 批量执行 MQL 脚本文件
//! - 静默模式和结果格式化
//! - 注释过滤(-- 和 // 风格)

use crate::client::Client;
use crate::formatter::Formatter;
use crate::{CliResult, Config};
use std::fs;
use std::path::Path;

/// CLI 非交互模式执行器
///
/// 用于执行单条查询或批量脚本文件,不进入 REPL 环境。
pub struct Cli {
    /// 数据库客户端连接
    client: Client,
    /// 结果格式化器
    formatter: Formatter,
    /// 静默模式(不输出结果)
    quiet: bool,
}

impl Cli {
    /// # Brief
    /// 创建 CLI 执行器
    ///
    /// 连接到数据库并初始化格式化器。
    ///
    /// # Arguments
    /// * `config` - CLI 配置
    ///
    /// # Returns
    /// 初始化的 CLI 实例
    pub async fn new(config: Config) -> CliResult<Self> {
        // 连接到数据库
        let client = Client::connect(&config).await?;
        let formatter = Formatter::new(&config.format, config.color);

        Ok(Self {
            client,
            formatter,
            quiet: config.quiet,
        })
    }

    /// # Brief
    /// 执行单条 MQL 查询
    ///
    /// 发送查询到服务器并格式化输出结果(除非在静默模式)。
    ///
    /// # Arguments
    /// * `query` - MQL 查询语句
    ///
    /// # Returns
    /// 执行结果
    pub async fn execute(&mut self, query: &str) -> CliResult<()> {
        // 发送查询到服务器
        let result = self.client.query(query).await?;

        // 非静默模式下输出结果
        if !self.quiet {
            self.formatter.print(&result);
        }

        Ok(())
    }

    /// # Brief
    /// 执行 MQL 脚本文件
    ///
    /// 逐行读取文件并执行查询,支持以下特性:
    /// - 跳过空行
    /// - 跳过注释行(-- 或 // 开头)
    /// - 非静默模式下显示正在执行的语句
    ///
    /// # Arguments
    /// * `path` - 脚本文件路径
    ///
    /// # Returns
    /// 执行结果(遇到错误立即停止)
    pub async fn execute_file(&mut self, path: &Path) -> CliResult<()> {
        // 读取整个文件内容
        let content = fs::read_to_string(path)?;

        // 逐行处理
        for line in content.lines() {
            let line = line.trim();
            // 跳过空行和注释(SQL 风格 -- 或 C++ 风格 //)
            if line.is_empty() || line.starts_with("--") || line.starts_with("//") {
                continue;
            }

            // 非静默模式下显示正在执行的语句
            if !self.quiet {
                println!("> {}", line);
            }

            // 执行查询,遇到错误立即返回
            self.execute(line).await?;
        }

        Ok(())
    }
}