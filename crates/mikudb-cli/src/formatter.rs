use colored::Colorize;
use serde_json::Value;

pub struct Formatter {
    format: OutputFormat,
    color: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Table,
    Json,
    JsonPretty,
    Csv,
    Line,
}

impl Formatter {
    pub fn new(format: &str, color: bool) -> Self {
        let format = match format.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "jsonpretty" | "json-pretty" => OutputFormat::JsonPretty,
            "csv" => OutputFormat::Csv,
            "line" => OutputFormat::Line,
            _ => OutputFormat::Table,
        };

        Self { format, color }
    }

    pub fn print(&self, result: &QueryResult) {
        if !result.success {
            if let Some(msg) = &result.message {
                eprintln!("{} {}", "Error:".red().bold(), msg);
            }
            return;
        }

        if result.documents.is_empty() {
            if let Some(msg) = &result.message {
                println!("{}", msg);
            } else {
                println!("{}", "No documents found.".dimmed());
            }
            self.print_affected(result.affected);
            return;
        }

        match self.format {
            OutputFormat::Table => self.print_table(&result.documents),
            OutputFormat::Json => self.print_json(&result.documents, false),
            OutputFormat::JsonPretty => self.print_json(&result.documents, true),
            OutputFormat::Csv => self.print_csv(&result.documents),
            OutputFormat::Line => self.print_line(&result.documents),
        }

        self.print_affected(result.affected);
    }

    fn print_table(&self, documents: &[Value]) {
        if documents.is_empty() {
            return;
        }

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

        if columns.contains(&"_id".to_string()) {
            columns.retain(|c| c != "_id");
            columns.insert(0, "_id".to_string());
        }

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

    fn print_json(&self, documents: &[Value], pretty: bool) {
        let output = if documents.len() == 1 {
            if pretty {
                serde_json::to_string_pretty(&documents[0])
            } else {
                serde_json::to_string(&documents[0])
            }
        } else {
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

    fn print_csv(&self, documents: &[Value]) {
        if documents.is_empty() {
            return;
        }

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

        println!("{}", columns.join(","));

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

    fn print_line(&self, documents: &[Value]) {
        for (i, doc) in documents.iter().enumerate() {
            if i > 0 {
                println!("{}", "-".repeat(40));
            }
            if let Value::Object(map) = doc {
                for (key, value) in map {
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

    fn print_affected(&self, affected: u64) {
        if affected > 0 {
            let msg = format!("{} document(s) affected", affected);
            if self.color {
                println!("{}", msg.dimmed());
            } else {
                println!("{}", msg);
            }
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn print_simple_table(headers: &[String], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = headers.iter().map(|h| strip_ansi(h).len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let separator: String = widths.iter().map(|w| "-".repeat(*w + 2)).collect::<Vec<_>>().join("+");

    let header_row: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!(" {:width$} ", h, width = widths.get(i).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("|");

    println!("+{}+", separator);
    println!("|{}|", header_row);
    println!("+{}+", separator);

    for row in rows {
        let row_str: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| format!(" {:width$} ", cell, width = widths.get(i).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join("|");
        println!("|{}|", row_str);
    }

    println!("+{}+", separator);
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub success: bool,
    pub affected: u64,
    pub documents: Vec<Value>,
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
