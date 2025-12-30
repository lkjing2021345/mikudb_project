use crate::client::Client;
use crate::formatter::Formatter;
use crate::{CliResult, Config};
use std::fs;
use std::path::Path;

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
    let q = query.trim_start();
    let first = q
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    let is_sql = matches!(
        first.as_str(),
        "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE"
            | "CREATE" | "DROP" | "ALTER" | "TRUNCATE"
            | "GRANT" | "REVOKE"
            | "BEGIN" | "COMMIT" | "ROLLBACK"
            | "EXPLAIN" | "DESCRIBE" | "DESC" 
    );
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
