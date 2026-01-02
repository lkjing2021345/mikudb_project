//! MQL 命令自动补全模块
//!
//! 本模块实现了 MikuDB CLI 的 Tab 自动补全功能,支持:
//! - MQL 关键字补全
//! - 上下文感知补全(如 SHOW 后面自动提示 DATABASE/COLLECTION 等)
//! - UTF-8 安全的字符串处理

use rustyline::Result;

/// MQL 自动补全器
///
/// 提供 MQL 关键字和命令的自动补全功能。
/// 使用 UTF-8 安全的字符边界处理,避免多字节字符导致的崩溃。
pub struct MqlCompleter {
    /// MQL 关键字列表(大写)
    keywords: Vec<&'static str>,
    /// CLI 内置命令列表(小写)
    commands: Vec<&'static str>,
}

impl MqlCompleter {
    /// # Brief
    /// 创建新的 MQL 补全器
    ///
    /// # Returns
    /// 初始化好关键字和命令列表的补全器实例
    pub fn new() -> Self {
        Self {
            keywords: vec![
                // CRUD 操作
                "FIND", "INSERT", "UPDATE", "DELETE", "INTO", "FROM", "WHERE",
                "SET", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
                // 查询子句
                "SELECT", "ORDER", "BY", "ASC", "DESC", "LIMIT", "SKIP", "OFFSET",
                // DDL 操作
                "CREATE", "DROP", "ALTER", "INDEX", "COLLECTION", "DATABASE",
                // 管理命令
                "SHOW", "USE", "STATUS", "USERS", "USER",
                // 事务
                "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
                // 聚合操作
                "AGGREGATE", "MATCH", "GROUP", "SORT", "PROJECT", "LOOKUP",
                "UNWIND", "BUCKET", "AS", "ON", "UNIQUE", "TEXT", "TTL",
                // 字面量
                "TRUE", "FALSE",
            ],
            commands: vec![
                "help", "exit", "quit", "clear", "status", "use",
            ],
        }
    }

    /// # Brief
    /// 执行自动补全
    ///
    /// 根据当前输入位置和上下文,生成候选补全列表。
    /// 使用 UTF-8 安全的 char_indices() 而非字节索引,避免多字节字符崩溃。
    ///
    /// # Arguments
    /// * `line` - 当前输入行
    /// * `pos` - 光标位置(字节偏移)
    ///
    /// # Returns
    /// (替换起始位置, 候选补全列表)
    pub fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<String>)> {
        // 获取光标之前的文本
        let line_to_cursor = if pos <= line.len() {
            &line[..pos]
        } else {
            line
        };

        // 使用 char_indices() 查找单词起始位置,确保 UTF-8 安全
        // 这里不能使用 rfind() + 1,因为 +1 可能落在多字节字符中间导致 panic
        let word_start = line_to_cursor
            .char_indices()  // 遍历字符及其字节索引
            .rev()  // 从后向前查找
            .find(|(_, c)| c.is_whitespace() || *c == '(' || *c == '{' || *c == '[' || *c == ',')
            .map(|(i, c)| i + c.len_utf8())  // 使用字符的 UTF-8 长度,而非固定的 +1
            .unwrap_or(0);

        // 提取当前单词前缀
        let prefix = if word_start <= line_to_cursor.len() {
            &line_to_cursor[word_start..]
        } else {
            ""
        };

        // 空前缀不补全
        if prefix.is_empty() {
            return Ok((pos, vec![]));
        }

        let prefix_upper = prefix.to_uppercase();
        let mut matches: Vec<String> = Vec::new();

        // 匹配关键字
        for &keyword in &self.keywords {
            if keyword.starts_with(&prefix_upper) {
                // 根据用户输入的大小写风格返回对应的补全
                if prefix.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                    matches.push(keyword.to_lowercase());
                } else {
                    matches.push(keyword.to_string());
                }
            }
        }

        // 如果在行首,也匹配内置命令
        if word_start == 0 {
            for &cmd in &self.commands {
                if cmd.starts_with(&prefix.to_lowercase()) {
                    matches.push(cmd.to_string());
                }
            }
        }

        // 添加上下文感知补全
        let context_completions = self.context_completions(line_to_cursor, &prefix_upper);
        matches.extend(context_completions);

        // 去重和排序
        matches.sort();
        matches.dedup();

        Ok((word_start, matches))
    }

    /// # Brief
    /// 根据上下文生成补全建议
    ///
    /// 识别特定关键字后的上下文,提供相关的补全选项。
    /// 例如:"SHOW " 后面提示 DATABASE/COLLECTION 等。
    ///
    /// # Arguments
    /// * `line` - 当前输入行
    /// * `_prefix` - 当前单词前缀(未使用)
    ///
    /// # Returns
    /// 上下文相关的补全列表
    fn context_completions(&self, line: &str, _prefix: &str) -> Vec<String> {
        let upper = line.to_uppercase();
        let mut completions = Vec::new();

        // SHOW 命令的上下文补全
        if upper.ends_with("SHOW ") {
            completions.extend(vec![
                "DATABASE".to_string(),
                "COLLECTION".to_string(),
                "INDEX".to_string(),
                "STATUS".to_string(),
                "USERS".to_string(),
            ]);
        }

        // CREATE 命令的上下文补全
        if upper.ends_with("CREATE ") {
            completions.extend(vec![
                "COLLECTION".to_string(),
                "DATABASE".to_string(),
                "INDEX".to_string(),
                "UNIQUE".to_string(),
                "USER".to_string(),
            ]);
        }

        // DROP 命令的上下文补全
        if upper.ends_with("DROP ") {
            completions.extend(vec![
                "COLLECTION".to_string(),
                "DATABASE".to_string(),
                "INDEX".to_string(),
                "USER".to_string(),
            ]);
        }

        // ORDER 和 GROUP 后面提示 BY
        if upper.ends_with("ORDER ") {
            completions.push("BY".to_string());
        }

        if upper.ends_with("GROUP ") {
            completions.push("BY".to_string());
        }

        // INSERT 后面提示 INTO
        if upper.ends_with("INSERT ") {
            completions.push("INTO".to_string());
        }

        // DELETE 后面提示 FROM
        if upper.ends_with("DELETE ") {
            completions.push("FROM".to_string());
        }

        completions
    }

    /// # Brief
    /// 添加集合名称到补全列表(预留接口)
    ///
    /// # Arguments
    /// * `_name` - 集合名称
    pub fn add_collection(&mut self, _name: &str) {
        // TODO: 实现动态集合名称补全
    }

    /// # Brief
    /// 添加字段名称到补全列表(预留接口)
    ///
    /// # Arguments
    /// * `_name` - 字段名称
    pub fn add_field(&mut self, _name: &str) {
        // TODO: 实现动态字段名称补全
    }
}

impl Default for MqlCompleter {
    fn default() -> Self {
        Self::new()
    }
}
