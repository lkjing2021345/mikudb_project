use crate::client::Client;
use crate::formatter::Formatter;
use crate::{CliError, CliResult, Config};
use std::path::Path;
use std::fs;

pub struct Cli {
    client: Client,
    formatter: Formatter,
    quiet: bool,
}

impl Cli {
    pub async fn new(config: Config) -> CliResult<Self> {
        let client = Client::connect(&config).await?;
        let formatter = Formatter::new(&config.format, config.color);

        Ok(Self {
            client,
            formatter,
            quiet: config.quiet,
        })
    }

pub async fn execute(&mut self, query: &str) -> CliResult<()> {
    // 1) 预检查：拦截常见 SQL 关键字，避免误报 early eof
    let q = query.trim_start();

    // 取第一段 token（直到空白），用来判断语句类型
    let first = q
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();

    // 常见 SQL 关键字（按需可继续补充）
    let is_sql = matches!(
        first.as_str(),
        "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE"
            | "CREATE" | "DROP" | "ALTER" | "TRUNCATE"
            | "GRANT" | "REVOKE"
            | "BEGIN" | "COMMIT" | "ROLLBACK"
            | "EXPLAIN" | "DESCRIBE" | "DESC" 
    );

    // 额外处理：即使用户前面带了分号或括号，也尽量识别
    // 例如 ";SELECT 1" 或 "(SELECT 1)"
    let first_trimmed = first.trim_matches(|c: char| !c.is_ascii_alphabetic());
    let is_sql = is_sql
        || matches!(
            first_trimmed,
            "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE"
                | "CREATE" | "DROP" | "ALTER" | "TRUNCATE"
                | "GRANT" | "REVOKE"
                | "BEGIN" | "COMMIT" | "ROLLBACK"
                | "EXPLAIN" | "DESCRIBE" | "DESC" 
        );

    if is_sql {
        return Err(CliError::Other(
            format!(
                "MikuDB CLI does not support SQL. You entered a statement starting with {}.\n\
Try MQL examples:\n\
  SHOW DATABASES\n\
  USE mydb\n\
  CREATE COLLECTION users\n\
  INSERT INTO users {{\"name\":\"alice\",\"age\":18}}\n\
  FIND users\n",
                first_trimmed
            ),
        ));
    }

    // 2) 正常执行
    let result = self.client.query(query).await?;

    if !self.quiet {
        self.formatter.print(&result);
    }

    Ok(())
}

pub async fn execute_file(&mut self, path: &Path) -> CliResult<()> {
        let content = fs::read_to_string(path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("--") || line.starts_with("//") {
                continue;
            }

            if !self.quiet {
                println!("> {}", line);
            }

            self.execute(line).await?;
        }

        Ok(())
    }
}
