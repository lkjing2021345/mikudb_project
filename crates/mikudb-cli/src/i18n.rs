//! 国际化 (i18n) 模块
//!
//! 提供多语言支持,支持中文和英文切换。

use std::sync::RwLock;
use std::path::PathBuf;
use std::fs;

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// 英语
    English,
    /// 中文
    Chinese,
}

impl Language {
    /// 从字符串解析语言
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "en" | "english" | "英文" => Some(Language::English),
            "zh" | "cn" | "chinese" | "中文" => Some(Language::Chinese),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }

    /// 转换为配置值
    pub fn to_config_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }
}

/// 全局语言设置
static CURRENT_LANGUAGE: RwLock<Language> = RwLock::new(Language::English);

/// 获取当前语言
pub fn current_language() -> Language {
    *CURRENT_LANGUAGE.read().unwrap()
}

/// 设置当前语言
pub fn set_language(lang: Language) {
    *CURRENT_LANGUAGE.write().unwrap() = lang;
    let _ = save_language_config(lang);
}

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".mikudb_config"))
        .unwrap_or_else(|| PathBuf::from(".mikudb_config"))
}

/// 保存语言配置
fn save_language_config(lang: Language) -> std::io::Result<()> {
    let config_path = get_config_path();
    fs::write(config_path, lang.to_config_str())
}

/// 加载语言配置
pub fn load_language_config() {
    if let Ok(content) = fs::read_to_string(get_config_path()) {
        if let Some(lang) = Language::from_str(content.trim()) {
            *CURRENT_LANGUAGE.write().unwrap() = lang;
        }
    }
}

/// 翻译宏
macro_rules! t {
    ($key:expr) => {
        crate::i18n::translate($key)
    };
}

pub(crate) use t;

/// 翻译函数
pub fn translate(key: &str) -> &'static str {
    match current_language() {
        Language::English => translate_en(key),
        Language::Chinese => translate_zh(key),
    }
}

/// 英文翻译
fn translate_en(key: &str) -> &'static str {
    match key {
        // 欢迎信息
        "welcome.title" => "Welcome to MikuDB Shell",
        "welcome.version" => "Version",
        "welcome.help" => "Type 'help' for usage information",

        // 命令提示
        "prompt.not_connected" => "not connected",

        // 帮助信息
        "help.title" => "MikuDB Command Help",
        "help.builtin" => "Built-in Commands:",
        "help.exit" => "Exit the shell",
        "help.help" => "Show this help message",
        "help.clear" => "Clear the screen",
        "help.use" => "Switch to database",
        "help.status" => "Show connection status",
        "help.lang" => "Switch language (en/zh)",
        "help.mql" => "MQL Commands:",
        "help.find" => "Query documents",
        "help.insert" => "Insert document",
        "help.update" => "Update documents",
        "help.delete" => "Delete documents",
        "help.create" => "Create collection/database/index",
        "help.drop" => "Drop collection/database/index",
        "help.show" => "Show collections/databases/status",
        "help.examples" => "Examples:",
        "help.example1" => "Find all users",
        "help.example2" => "Insert a document",
        "help.example3" => "Update documents",

        // 状态信息
        "status.title" => "Connection Status",
        "status.server" => "Server",
        "status.connected" => "Connected",
        "status.database" => "Current Database",
        "status.user" => "User",
        "status.format" => "Output Format",

        // 错误信息
        "error.unknown_command" => "Unknown command",
        "error.use_syntax" => "Usage: USE <database>",
        "error.no_database" => "No database selected. Use 'USE <database>' first.",
        "error.query_failed" => "Query failed",
        "error.connection_lost" => "Connection lost",

        // 结果信息
        "result.no_documents" => "No documents found.",
        "result.affected" => "affected",
        "result.document" => "document",
        "result.documents" => "documents",

        // 语言切换
        "lang.switched" => "Language switched to",
        "lang.current" => "Current language",
        "lang.usage" => "Usage: LANG <en|zh>",

        _ => "",
    }
}

/// 中文翻译
fn translate_zh(key: &str) -> &'static str {
    match key {
        // 欢迎信息
        "welcome.title" => "欢迎使用 MikuDB 交互式命令行",
        "welcome.version" => "版本",
        "welcome.help" => "输入 'help' 查看帮助信息",

        // 命令提示
        "prompt.not_connected" => "未连接",

        // 帮助信息
        "help.title" => "MikuDB 命令帮助",
        "help.builtin" => "内置命令:",
        "help.exit" => "退出命令行",
        "help.help" => "显示帮助信息",
        "help.clear" => "清空屏幕",
        "help.use" => "切换数据库",
        "help.status" => "显示连接状态",
        "help.lang" => "切换语言 (en/zh)",
        "help.mql" => "MQL 查询命令:",
        "help.find" => "查询文档",
        "help.insert" => "插入文档",
        "help.update" => "更新文档",
        "help.delete" => "删除文档",
        "help.create" => "创建集合/数据库/索引",
        "help.drop" => "删除集合/数据库/索引",
        "help.show" => "显示集合/数据库/状态",
        "help.examples" => "示例:",
        "help.example1" => "查找所有用户",
        "help.example2" => "插入一个文档",
        "help.example3" => "更新文档",

        // 状态信息
        "status.title" => "连接状态",
        "status.server" => "服务器",
        "status.connected" => "已连接",
        "status.database" => "当前数据库",
        "status.user" => "用户",
        "status.format" => "输出格式",

        // 错误信息
        "error.unknown_command" => "未知命令",
        "error.use_syntax" => "用法: USE <数据库名>",
        "error.no_database" => "未选择数据库,请先使用 'USE <数据库名>'",
        "error.query_failed" => "查询失败",
        "error.connection_lost" => "连接断开",

        // 结果信息
        "result.no_documents" => "未找到文档",
        "result.affected" => "受影响",
        "result.document" => "文档",
        "result.documents" => "文档",

        // 语言切换
        "lang.switched" => "语言已切换到",
        "lang.current" => "当前语言",
        "lang.usage" => "用法: LANG <en|zh>",

        _ => "",
    }
}
