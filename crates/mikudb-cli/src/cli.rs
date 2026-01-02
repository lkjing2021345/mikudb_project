use crate::client::Client;
use crate::formatter::Formatter;
use crate::{CliResult, Config, CliError};
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
