//! MQL 语法高亮模块
//!
//! 本模块实现 MQL 查询语句的语法高亮:
//! - 关键字高亮 (FIND, INSERT, WHERE 等)
//! - 函数高亮 (COUNT, SUM, AVG 等)
//! - 字符串、数字、操作符着色
//! - 括号和特殊字符标记
//! - 转义字符处理

use colored::Colorize;

/// MQL 语法高亮器
///
/// 为 MQL 语句提供彩色输出,提升可读性。
pub struct MqlHighlighter {
    /// MQL 关键字列表
    keywords: Vec<&'static str>,
    /// 内置函数列表
    functions: Vec<&'static str>,
    /// 操作符列表
    operators: Vec<&'static str>,
}

impl MqlHighlighter {
    /// # Brief
    /// 创建语法高亮器
    ///
    /// 初始化所有 MQL 关键字、函数和操作符。
    pub fn new() -> Self {
        Self {
            // MQL 关键字(数据操作、DDL、事务、聚合等)
            keywords: vec![
                "FIND", "INSERT", "UPDATE", "DELETE", "INTO", "FROM", "WHERE",
                "SET", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
                "SELECT", "ORDER", "BY", "ASC", "DESC", "LIMIT", "SKIP", "OFFSET",
                "CREATE", "DROP", "ALTER", "INDEX", "COLLECTION", "DATABASE",
                "SHOW", "USE", "DATABASES", "COLLECTIONS", "INDEXES",
                "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
                "AGGREGATE", "MATCH", "GROUP", "SORT", "PROJECT", "LOOKUP",
                "UNWIND", "BUCKET", "AS", "ON", "UNIQUE", "TEXT", "TTL",
                "TRUE", "FALSE",
            ],
            // 内置函数(聚合、更新操作符、日期、字符串等)
            functions: vec![
                "COUNT", "SUM", "AVG", "MIN", "MAX", "FIRST", "LAST",
                "PUSH", "PULL", "ADDTOSET", "POP", "UNSET", "INC", "MUL",
                "NOW", "DATE", "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND",
                "UPPER", "LOWER", "TRIM", "SUBSTR", "CONCAT", "SPLIT",
                "SIZE", "TYPE", "OBJECTID",
            ],
            // 比较和算术操作符
            operators: vec![
                "=", "!=", "<>", "<", ">", "<=", ">=", "+", "-", "*", "/", "%",
            ],
        }
    }

    /// # Brief
    /// 高亮 MQL 语句
    ///
    /// 解析输入字符串,识别不同的语法元素并应用颜色:
    /// - 关键字: 蓝色加粗
    /// - 函数: 青色
    /// - 字符串: 黄色
    /// - 数字: 亮品红
    /// - 操作符: 红色
    /// - 括号: 品红加粗
    /// - $ 字段: 绿色
    ///
    /// # Arguments
    /// * `input` - 原始 MQL 语句
    ///
    /// # Returns
    /// 带 ANSI 颜色代码的字符串
    pub fn highlight(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        let mut current_word = String::new();

        // 逐字符解析,识别不同的语法元素
        while let Some(ch) = chars.next() {
            // 处理字符串字面量(单引号或双引号)
            if ch == '"' || ch == '\'' {
                // 先输出当前单词
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }

                let quote = ch;
                let mut string_content = String::new();
                string_content.push(quote);

                // 读取直到遇到配对的引号
                while let Some(c) = chars.next() {
                    string_content.push(c);
                    if c == quote {
                        break;  // 遇到配对引号,字符串结束
                    }
                    // 处理转义字符(如 \n, \t, \")
                    if c == '\\' {
                        if let Some(escaped) = chars.next() {
                            string_content.push(escaped);
                        }
                    }
                }

                // 字符串高亮为黄色
                result.push_str(&string_content.yellow().to_string());
            // 处理括号(用于 JSON 对象和数组)
            } else if ch == '{' || ch == '}' || ch == '[' || ch == ']' {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                // 括号高亮为品红加粗
                result.push_str(&ch.to_string().magenta().bold().to_string());
            // 处理分隔符
            } else if ch == ':' || ch == ',' {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                result.push(ch);
            // 处理空白符(空格、换行等)
            } else if ch.is_whitespace() {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                result.push(ch);
            // 处理操作符(可能是多字符,如 !=, <=)
            } else if self.is_operator_char(ch) {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }

                // 贪婪匹配多字符操作符
                let mut op = String::new();
                op.push(ch);
                while let Some(&next) = chars.peek() {
                    if self.is_operator_char(next) {
                        op.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                // 操作符高亮为红色
                result.push_str(&op.red().to_string());
            // 普通字符,累积到当前单词
            } else {
                current_word.push(ch);
            }
        }

        // 输出最后一个单词
        if !current_word.is_empty() {
            result.push_str(&self.highlight_word(&current_word));
        }

        result
    }

    /// # Brief
    /// 高亮单个单词
    ///
    /// 根据单词类型应用不同颜色:
    /// - MQL 关键字: 蓝色加粗
    /// - 内置函数: 青色
    /// - $ 开头字段: 绿色
    /// - 数字: 亮品红
    /// - 其他: 无颜色
    ///
    /// # Arguments
    /// * `word` - 待高亮的单词
    ///
    /// # Returns
    /// 带颜色的字符串
    fn highlight_word(&self, word: &str) -> String {
        let upper = word.to_uppercase();

        // 检查是否为关键字
        if self.keywords.contains(&upper.as_str()) {
            return word.blue().bold().to_string();
        }

        // 检查是否为内置函数
        if self.functions.contains(&upper.as_str()) {
            return word.cyan().to_string();
        }

        // $ 开头的字段名(MongoDB 风格)
        if word.starts_with('$') {
            return word.green().to_string();
        }

        // 数字字面量
        if word.parse::<f64>().is_ok() {
            return word.bright_magenta().to_string();
        }

        // 普通文本,不着色
        word.to_string()
    }

    /// # Brief
    /// 判断字符是否为操作符字符
    ///
    /// 操作符可能由多个字符组成,如 !=, <=, >=。
    ///
    /// # Arguments
    /// * `ch` - 待检查的字符
    ///
    /// # Returns
    /// true 表示是操作符字符
    fn is_operator_char(&self, ch: char) -> bool {
        matches!(ch, '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%')
    }
}

impl Default for MqlHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
