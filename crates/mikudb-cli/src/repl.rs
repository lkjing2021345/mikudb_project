//! REPL 交互式环境模块
//!
//! 本模块实现 MikuDB 的交互式命令行界面:
//! - Rustyline 基础的命令行编辑
//! - 语法高亮和自动补全
//! - 命令历史管理
//! - 内置命令处理 (help, exit, status 等)
//! - 输入验证(括号匹配)

use crate::client::Client;
use crate::completer::MqlCompleter;
use crate::formatter::Formatter;
use crate::help;
use crate::highlighter::MqlHighlighter;
use crate::i18n::{current_language, set_language, t, Language};
use crate::{CliError, CliResult, Config};
use colored::Colorize;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, EditMode, Editor};
use std::borrow::Cow;

/// REPL 交互式环境
///
/// 管理客户端连接、命令行编辑器和结果格式化。
pub struct Repl {
    /// 数据库客户端连接
    client: Client,
    /// 结果格式化器
    formatter: Formatter,
    /// Rustyline 编辑器(带自动补全和高亮)
    editor: Editor<MqlHelper, DefaultHistory>,
    /// 当前数据库
    current_database: Option<String>,
    /// 历史记录文件路径
    history_file: String,
}

/// Rustyline Helper
///
/// 整合自动补全、语法高亮、提示和验证功能。
#[derive(rustyline_derive::Helper)]
struct MqlHelper {
    /// MQL 自动补全器
    completer: MqlCompleter,
    /// MQL 语法高亮器
    highlighter: MqlHighlighter,
}

// 实现 Rustyline 自动补全接口
impl rustyline::completion::Completer for MqlHelper {
    type Candidate = String;

    /// # Brief
    /// 执行自动补全
    ///
    /// 委托给 MqlCompleter 处理。
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        self.completer.complete(line, pos)
    }
}

// 实现 Rustyline 提示接口
impl rustyline::hint::Hinter for MqlHelper {
    type Hint = String;

    /// # Brief
    /// 生成输入提示
    ///
    /// 根据当前输入提供 MQL 命令提示。
    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        // 只在光标在末尾时显示提示
        if line.is_empty() || pos < line.len() {
            return None;
        }

        // 常用 MQL 命令提示模板
        let hints = [
            ("FIND", " collection_name WHERE field = value"),
            ("INSERT", " INTO collection_name {field: value}"),
            ("UPDATE", " collection_name SET field = value WHERE condition"),
            ("DELETE", " FROM collection_name WHERE condition"),
            ("CREATE", " COLLECTION collection_name"),
            ("DROP", " COLLECTION collection_name"),
            ("SHOW", " COLLECTION"),
            ("USE", " database_name"),
        ];

        let upper = line.to_uppercase();
        // 匹配部分输入或完整命令
        for (prefix, hint) in hints {
            if prefix.starts_with(&upper) && upper.len() < prefix.len() {
                return Some(format!("{}{}", &prefix[upper.len()..], hint));
            }
            if upper == prefix {
                return Some(hint.to_string());
            }
        }

        None
    }
}

// 实现 Rustyline 语法高亮接口
impl rustyline::highlight::Highlighter for MqlHelper {
    /// # Brief
    /// 高亮输入行
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(self.highlighter.highlight(line))
    }

    /// # Brief
    /// 高亮提示符 (绿色加粗)
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Owned(prompt.green().bold().to_string())
    }

    /// # Brief
    /// 高亮提示文本 (淡化显示)
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(hint.dimmed().to_string())
    }

    /// # Brief
    /// 启用实时高亮
    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

// 实现 Rustyline 输入验证接口
impl rustyline::validate::Validator for MqlHelper {
    /// # Brief
    /// 验证输入是否完整
    ///
    /// 检查括号和方括号是否匹配,不匹配则继续读取下一行。
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        let input = ctx.input();

        // 计算括号数量
        let open_braces = input.matches('{').count();
        let close_braces = input.matches('}').count();
        let open_brackets = input.matches('[').count();
        let close_brackets = input.matches(']').count();

        // 括号不匹配则表示输入未完成
        if open_braces != close_braces || open_brackets != close_brackets {
            Ok(rustyline::validate::ValidationResult::Incomplete)
        } else {
            Ok(rustyline::validate::ValidationResult::Valid(None))
        }
    }
}

impl Repl {
    /// # Brief
    /// 创建 REPL 环境
    ///
    /// 连接到数据库,初始化编辑器和历史记录。
    ///
    /// # Arguments
    /// * `config` - CLI 配置
    ///
    /// # Returns
    /// 初始化的 REPL 实例
    pub async fn new(config: Config) -> CliResult<Self> {
        // 连接到数据库
        let client = Client::connect(&config).await?;
        let formatter = Formatter::new(&config.format, config.color);

        // 创建 MQL Helper (补全 + 高亮)
        let helper = MqlHelper {
            completer: MqlCompleter::new(),
            highlighter: MqlHighlighter::new(),
        };

        // 初始化 Rustyline 编辑器
        let mut editor = Editor::new().map_err(|e| CliError::Other(e.to_string()))?;
        editor.set_helper(Some(helper));
        editor.set_completion_type(CompletionType::List);  // 列表式补全
        editor.set_edit_mode(EditMode::Emacs);              // Emacs 编辑模式

        // 设置历史记录文件路径
        let history_file = dirs::home_dir()
            .map(|h| h.join(".mikudb_history"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".mikudb_history".to_string());

        // 加载历史记录
        let _ = editor.load_history(&history_file);

        Ok(Self {
            client,
            formatter,
            editor,
            current_database: config.database,
            history_file,
        })
    }

    /// # Brief
    /// 运行 REPL 主循环
    ///
    /// 循环读取用户输入,执行命令,显示结果。
    pub async fn run(&mut self) -> CliResult<()> {
        self.print_welcome();

        loop {
            let prompt = self.get_prompt();

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // 添加到历史记录
                    let _ = self.editor.add_history_entry(line);

                    // 处理 ? 后缀帮助
                    if line.ends_with('?') {
                        let cmd = line.trim_end_matches('?').trim();
                        if let Some(help_text) = help::get_command_help(cmd) {
                            println!("{}", help_text);
                        } else {
                            println!("{} {}", "No help available for:".yellow(), cmd);
                        }
                        continue;
                    }

                    // 处理内置命令
                    if self.handle_builtin(line).await? {
                        continue;
                    }

                    // 执行 MQL 查询
                    match self.client.query(line).await {
                        Ok(result) => {
                            self.formatter.print(&result);
                        }
                        Err(e) => {
                            eprintln!("{} {}", "Error:".red().bold(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D
                    println!("Bye!");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        // 保存历史记录
        let _ = self.editor.save_history(&self.history_file);
        Ok(())
    }

    /// # Brief
    /// 打印欢迎信息
    fn print_welcome(&self) {
        println!(
            r#"
  __  __ _ _          ____  ____
 |  \/  (_) | ___   _|  _ \| __ )
 | |\/| | | |/ / | | | | | |  _ \
 | |  | | |   <| |_| | |_| | |_) |
 |_|  |_|_|_|\_\\__,_|____/|____/
"#
        );
        println!(" {} v{}", t!("welcome.title"), env!("CARGO_PKG_VERSION"));
        println!(" {}", t!("welcome.help"));
        println!();
        println!(" [Auth] Logged in as: {}", self.client.user().green().bold());
        if let Some(db) = &self.current_database {
            println!(" [DB] Current database: {}", db.cyan());
        }
        println!();
    }

    /// # Brief
    /// 生成命令行提示符
    ///
    /// 格式: "mikudb:database_name> " 或 "mikudb> "
    fn get_prompt(&self) -> String {
        match &self.current_database {
            Some(db) => format!("mikudb:{}> ", db.cyan()),
            None => "mikudb> ".to_string(),
        }
    }

    /// # Brief
    /// 处理内置命令
    ///
    /// 支持: exit, quit, help, clear, use, status
    ///
    /// # Returns
    /// true 表示命令已处理,false 表示需要发送到服务器
    async fn handle_builtin(&mut self, line: &str) -> CliResult<bool> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0].to_lowercase().as_str() {
            "exit" | "quit" | "\\q" => {
                println!("Bye!");
                std::process::exit(0);
            }
            "help" | "\\h" | "?" => {
                help::print_main_help();
                Ok(true)
            }
            "clear" | "\\c" => {
                print!("\x1B[2J\x1B[1;1H");
                Ok(true)
            }
            "use" => {
                if parts.len() > 1 {
                    self.current_database = Some(parts[1].to_string());
                    println!("Switched to database {}", parts[1].cyan());
                } else {
                    println!("Usage: use <database>");
                }
                Ok(true)
            }
            "status" | "\\s" => {
                self.print_status().await;
                Ok(true)
            }
            "passwd" | "password" => {
                self.change_password().await?;
                Ok(true)
            }
            "whoami" => {
                println!("Current user: {}", self.client.user().green().bold());
                Ok(true)
            }
            "lang" | "language" => {
                if parts.len() > 1 {
                    if let Some(lang) = Language::from_str(parts[1]) {
                        set_language(lang);
                        println!("{}: {}", t!("lang.switched"), lang.as_str());
                    } else {
                        println!("{}", t!("lang.usage"));
                    }
                } else {
                    println!("{}: {}", t!("lang.current"), current_language().as_str());
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn change_password(&self) -> CliResult<()> {
        use dialoguer::Password;

        println!("{}", "Change Password".green().bold());
        println!();

        let old_password = Password::new()
            .with_prompt("Current password")
            .interact()
            .map_err(|e| CliError::Other(format!("Failed to read password: {}", e)))?;

        let new_password = Password::new()
            .with_prompt("New password")
            .with_confirmation("Confirm new password", "Passwords do not match")
            .interact()
            .map_err(|e| CliError::Other(format!("Failed to read new password: {}", e)))?;

        if new_password.len() < 8 {
            println!("{}", "[X] Password must be at least 8 characters".red());
            return Ok(());
        }

        let query = format!(
            "ALTER USER \"{}\" PASSWORD \"{}\"",
            self.client.user(),
            new_password
        );

        match self.client.execute(&query).await {
            Ok(_) => {
                println!("{}", "[OK] Password changed successfully".green());
                println!("{}", "[!] Please reconnect with the new password".yellow());
            }
            Err(e) => {
                println!("{} {}", "[X] Failed to change password:".red(), e);
            }
        }

        Ok(())
    }

    /// # Brief
    /// 打印连接状态
    async fn print_status(&self) {
        println!("{}", t!("status.title").green().bold());
        println!("  {}: {}:{}", t!("status.server"), self.client.host(), self.client.port());
        println!("  {}: {}", t!("status.user"), self.client.user());
        println!(
            "  {}: {}",
            t!("status.database"),
            self.current_database
                .as_deref()
                .unwrap_or("(none)")
        );
        println!("  {}: {}", t!("status.connected"), t!("status.connected").green());
    }
}
