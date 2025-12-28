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
    let q = query.trim_start();

    if q.len() >= 6 && q[..6].eq_ignore_ascii_case("select") {
        return Err(CliError::Other(
            "MikuDB CLI does not support SQL. You entered a statement starting with SELECT.\n\
Try MQL examples:\n\
  SHOW DATABASES\n\
  USE mydb\n\
  CREATE COLLECTION users\n\
  INSERT INTO users {\"name\":\"alice\",\"age\":18}\n\
  FIND users\n"
                .to_string(),
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
