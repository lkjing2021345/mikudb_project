//! 查询结果格式化模块
//!
//! 本模块实现多种输出格式:
//! - Table: ASCII 表格格式(默认)
//! - JSON: 紧凑 JSON
//! - JSON Pretty: 格式化 JSON
//! - CSV: 逗号分隔值
//! - Line: 每个字段一行(适用于宽文档)
//!
//! 特殊处理:
//! - ObjectId 自动识别并格式化为十六进制
//! - 单个文档且字段 >8 时自动切换到 Line 格式
//! - ANSI 颜色支持

use crate::i18n::t;
use colored::Colorize;
use serde_json::Value;

/// 格式化器
///
/// 根据配置格式化查询结果并输出。
pub struct Formatter {
    /// 输出格式
    format: OutputFormat,
    /// 是否启用颜色
    color: bool,
}

/// 输出格式枚举
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// ASCII 表格格式
    Table,
    /// 紧凑 JSON
    Json,
    /// 格式化 JSON
    JsonPretty,
    /// CSV 格式
    Csv,
    /// 每行一个字段
    Line,
}

impl Formatter {
    /// # Brief
    /// 创建格式化器
    ///
    /// # Arguments
    /// * `format` - 格式名称("table", "json", "json-pretty", "csv", "line")
    /// * `color` - 是否启用 ANSI 颜色
    pub fn new(format: &str, color: bool) -> Self {
        let format = match format.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "jsonpretty" | "json-pretty" => OutputFormat::JsonPretty,
            "csv" => OutputFormat::Csv,
            "line" => OutputFormat::Line,
            _ => OutputFormat::Table,  // 默认为表格格式
        };

        Self { format, color }
    }

    /// # Brief
    /// 打印查询结果
    ///
    /// 根据配置的格式和颜色设置输出结果。
    ///
    /// # Arguments
    /// * `result` - 查询结果
    pub fn print(&self, result: &QueryResult) {
        // 处理错误情况
        if !result.success {
            if let Some(msg) = &result.message {
                eprintln!("{} {}", "Error:".red().bold(), msg);
            }
            return;
        }

        // 处理空结果集
        if result.documents.is_empty() {
            if let Some(msg) = &result.message {
                println!("{}", msg);
            } else {
                println!("{}", t!("result.no_documents").dimmed());
            }
            self.print_affected(result.affected);
            return;
        }

        // 自动切换到 Line 格式(单个文档且字段 >8)
        let use_line_format = if let OutputFormat::Table = self.format {
            if result.documents.len() == 1 {
                if let Value::Object(map) = &result.documents[0] {
                    map.len() > 8  // 超过 8 个字段时使用纵向显示
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // 选择格式化方法
        if use_line_format {
            self.print_line(&result.documents);
        } else {
            match self.format {
                OutputFormat::Table => self.print_table(&result.documents),
                OutputFormat::Json => self.print_json(&result.documents, false),
                OutputFormat::JsonPretty => self.print_json(&result.documents, true),
                OutputFormat::Csv => self.print_csv(&result.documents),
                OutputFormat::Line => self.print_line(&result.documents),
            }
        }

        self.print_affected(result.affected);
    }

    /// # Brief
    /// 打印 ASCII 表格
    ///
    /// 自动提取所有字段作为列,_id 字段自动移到第一列。
    ///
    /// # Arguments
    /// * `documents` - 文档数组
    fn print_table(&self, documents: &[Value]) {
        if documents.is_empty() {
            return;
        }

        // 提取所有字段名(合并所有文档的字段)
        let mut columns: Vec<String> = Vec::new();
        for doc in documents {
            if let Value::Object(map) = doc {
                for key in map.keys() {
                    if !columns.contains(key) {
                        columns.push(key.clone());
                    }
                }
            }
        }

        // 排序字段名
        columns.sort();

        // _id 字段移到最前
        if columns.contains(&"_id".to_string()) {
            columns.retain(|c| c != "_id");
            columns.insert(0, "_id".to_string());
        }

        // 构造表格数据
        let rows: Vec<Vec<String>> = documents
            .iter()
            .map(|doc| {
                columns
                    .iter()
                    .map(|col| {
                        if let Value::Object(map) = doc {
                            map.get(col)
                                .map(|v| format_value(v))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        }
                    })
                    .collect()
            })
            .collect();

        // 表头着色(青色加粗)
        let header: Vec<String> = columns.iter().map(|c| {
            if self.color {
                c.cyan().bold().to_string()
            } else {
                c.clone()
            }
        }).collect();

        println!();
        print_simple_table(&header, &rows);
        println!();
    }

    /// # Brief
    /// 打印 JSON 格式
    ///
    /// 单个文档时直接输出对象,多个文档时输出数组。
    ///
    /// # Arguments
    /// * `documents` - 文档数组
    /// * `pretty` - 是否格式化输出
    fn print_json(&self, documents: &[Value], pretty: bool) {
        let output = if documents.len() == 1 {
            // 单个文档直接输出对象
            if pretty {
                serde_json::to_string_pretty(&documents[0])
            } else {
                serde_json::to_string(&documents[0])
            }
        } else {
            // 多个文档输出数组
            if pretty {
                serde_json::to_string_pretty(documents)
            } else {
                serde_json::to_string(documents)
            }
        };

        if let Ok(json) = output {
            println!("{}", json);
        }
    }

    /// # Brief
    /// 打印 CSV 格式
    ///
    /// 第一行为列名,后续行为数据。处理逗号和引号转义。
    ///
    /// # Arguments
    /// * `documents` - 文档数组
    fn print_csv(&self, documents: &[Value]) {
        if documents.is_empty() {
            return;
        }

        // 提取所有字段名
        let mut columns: Vec<String> = Vec::new();
        for doc in documents {
            if let Value::Object(map) = doc {
                for key in map.keys() {
                    if !columns.contains(key) {
                        columns.push(key.clone());
                    }
                }
            }
        }
        columns.sort();

        // 打印表头
        println!("{}", columns.join(","));

        // 打印数据行
        for doc in documents {
            if let Value::Object(map) = doc {
                let row: Vec<String> = columns
                    .iter()
                    .map(|col| {
                        map.get(col)
                            .map(|v| csv_escape(&format_value(v)))
                            .unwrap_or_default()
                    })
                    .collect();
                println!("{}", row.join(","));
            }
        }
    }

    /// # Brief
    /// 打印行格式
    ///
    /// 每个字段占据一行,适用于字段多或内容宽的文档。
    ///
    /// # Arguments
    /// * `documents` - 文档数组
    fn print_line(&self, documents: &[Value]) {
        for (i, doc) in documents.iter().enumerate() {
            // 文档间用分割线
            if i > 0 {
                println!("{}", "-".repeat(40));
            }
            if let Value::Object(map) = doc {
                for (key, value) in map {
                    // 字段名着色(青色)
                    let key_str = if self.color {
                        key.cyan().to_string()
                    } else {
                        key.clone()
                    };
                    println!("{}: {}", key_str, format_value(value));
                }
            }
        }
    }

    /// # Brief
    /// 打印受影响文档数
    ///
    /// 用于 INSERT/UPDATE/DELETE 操作。
    ///
    /// # Arguments
    /// * `affected` - 受影响的文档数量
    fn print_affected(&self, affected: u64) {
        if affected > 0 {
            let doc_word = if affected == 1 {
                t!("result.document")
            } else {
                t!("result.documents")
            };
            let msg = format!("{} {} {}", affected, doc_word, t!("result.affected"));
            if self.color {
                println!("{}", msg.dimmed());
            } else {
                println!("{}", msg);
            }
        }
    }
}

/// # Brief
/// 格式化 JSON 值为字符串
///
/// 特殊处理:
/// - 12 字节数组识别为 ObjectId,格式化为十六进制
/// - 嵌套对象和数组递归处理
///
/// # Arguments
/// * `value` - JSON 值
///
/// # Returns
/// 格式化后的字符串
fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            // 特殊处理: 12 字节数组识别为 ObjectId
            if arr.len() == 12 && arr.iter().all(|v| v.is_u64()) {
                let bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                if bytes.len() == 12 {
                    // 格式化为十六进制字符串
                    return format!("{}", bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>());
                }
            }
            // 普通数组递归处理
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            // 嵌套对象递归处理
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

/// # Brief
/// CSV 转义
///
/// 包含逗号、引号或换行的字符串需要用双引号包裹。
///
/// # Arguments
/// * `s` - 原始字符串
///
/// # Returns
/// 转义后的字符串
fn csv_escape(s: &str) -> String {
    // 包含特殊字符则用双引号包裹,并将内部引号加倍
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// # Brief
/// 打印简单 ASCII 表格
///
/// 自动计算列宽,绘制边框和分隔线。
///
/// # Arguments
/// * `headers` - 表头
/// * `rows` - 数据行
fn print_simple_table(headers: &[String], rows: &[Vec<String>]) {
    // 计算每列最大宽度(strip ANSI 颜色代码)
    let mut widths: Vec<usize> = headers.iter().map(|h| strip_ansi(h).len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // 构造分隔线
    let separator: String = widths.iter().map(|w| "-".repeat(*w + 2)).collect::<Vec<_>>().join("+");

    // 构造表头行
    let header_row: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!(" {:width$} ", h, width = widths.get(i).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("|");

    // 打印表头
    println!("+{}+", separator);
    println!("|{}|", header_row);
    println!("+{}+", separator);

    // 打印数据行
    for row in rows {
        let row_str: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| format!(" {:width$} ", cell, width = widths.get(i).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join("|");
        println!("|{}|", row_str);
    }

    // 打印底部分隔线
    println!("+{}+", separator);
}

/// # Brief
/// 移除 ANSI 颜色代码
///
/// 用于计算字符串的真实宽度。
///
/// # Arguments
/// * `s` - 带 ANSI 代码的字符串
///
/// # Returns
/// 无颜色代码的字符串
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

/// 查询结果
///
/// 封装服务器返回的查询结果。
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// 查询是否成功
    pub success: bool,
    /// 受影响的文档数量
    pub affected: u64,
    /// 返回的文档数组
    pub documents: Vec<Value>,
    /// 消息(成功或错误提示)
    pub message: Option<String>,
}

impl Default for QueryResult {
    fn default() -> Self {
        Self {
            success: true,
            affected: 0,
            documents: vec![],
            message: None,
        }
    }
}
