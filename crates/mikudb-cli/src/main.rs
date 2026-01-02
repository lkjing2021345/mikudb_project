//! MikuDB CLI 主程序
//!
//! 命令行入口,支持三种模式:
//! - 交互式 REPL 模式(默认)
//! - 单条查询执行模式(-e 参数)
//! - 脚本文件执行模式(-f 参数)

use clap::Parser;
use mikudb_cli::{Cli, Config, Repl};
use std::path::PathBuf;

/// MikuDB CLI 命令行参数
#[derive(Parser, Debug)]
#[command(name = "mikudb-cli")]
#[command(author = "MikuDB Team")]
#[command(version)]
#[command(about = "MikuDB CLI - Interactive command-line client")]
struct Args {
    /// 服务器主机名
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    /// 服务器端口
    #[arg(short, long, default_value_t = 3939)]
    port: u16,

    /// 用户名
    #[arg(short, long)]
    user: Option<String>,

    /// 密码(未指定时交互式输入)
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// 默认数据库
    #[arg(short, long)]
    database: Option<String>,

    /// 执行单条查询后退出
    #[arg(short, long)]
    execute: Option<String>,

    /// 执行脚本文件后退出
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// 输出格式(table, json, json-pretty, csv, line)
    #[arg(long, default_value = "table")]
    format: String,

    /// 禁用颜色输出
    #[arg(long)]
    no_color: bool,

    /// 静默模式(不输出结果)
    #[arg(long)]
    quiet: bool,
}

/// # Brief
/// 主函数
///
/// 解析命令行参数,根据模式选择:
/// - 执行单条查询(-e)
/// - 执行脚本文件(-f)
/// - 进入 REPL 交互模式(默认)
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 CLI 环境(加载语言配置等)
    mikudb_cli::init();

    // 解析命令行参数
    let args = Args::parse();

    let user = match args.user {
        Some(u) => u,
        None => {
            if args.execute.is_some() || args.file.is_some() {
                return Err(anyhow::anyhow!("Username required in non-interactive mode. Use -u <username>"));
            }
            dialoguer::Input::new()
                .with_prompt("Username")
                .default("root".to_string())
                .interact_text()?
        }
    };

    let password = match args.password {
        Some(p) => p,
        None => {
            if args.execute.is_some() || args.file.is_some() {
                return Err(anyhow::anyhow!("Password required in non-interactive mode. Use -P <password>"));
            }
            dialoguer::Password::new()
                .with_prompt(format!("Password for {}", user))
                .interact()?
        }
    };

    let config = Config {
        host: args.host,
        port: args.port,
        user,
        password,
        database: args.database,
        format: args.format,
        color: !args.no_color,
        quiet: args.quiet,
    };

    // 单条查询模式
    if let Some(query) = args.execute {
        let mut cli = Cli::new(config).await?;
        cli.execute(&query).await?;
        return Ok(());
    }

    // 脚本文件模式
    if let Some(file) = args.file {
        let mut cli = Cli::new(config).await?;
        cli.execute_file(&file).await?;
        return Ok(());
    }

    // 默认进入 REPL 交互模式
    let mut repl = Repl::new(config).await?;
    repl.run().await?;

    Ok(())
}
