use clap::Parser;
use mikudb_cli::{Cli, Config, Repl};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mikudb-cli")]
#[command(author = "MikuDB Team")]
#[command(version)]
#[command(about = "MikuDB CLI - Interactive command-line client")]
struct Args {
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    #[arg(short, long, default_value_t = 3939)]
    port: u16,

    #[arg(short, long, default_value = "miku")]
    user: String,

    #[arg(short = 'P', long)]
    password: Option<String>,

    #[arg(short, long)]
    database: Option<String>,

    #[arg(short, long)]
    execute: Option<String>,

    #[arg(short, long)]
    file: Option<PathBuf>,

    #[arg(long, default_value = "table")]
    format: String,

    #[arg(long)]
    no_color: bool,

    #[arg(long)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let password = match args.password {
        Some(p) => p,
        None => {
            if args.execute.is_some() || args.file.is_some() {
                "mikumiku3939".to_string()
            } else {
                dialoguer::Password::new()
                    .with_prompt("Password")
                    .interact()?
            }
        }
    };

    let config = Config {
        host: args.host,
        port: args.port,
        user: args.user,
        password,
        database: args.database,
        format: args.format,
        color: !args.no_color,
        quiet: args.quiet,
    };

    if let Some(query) = args.execute {
        let mut cli = Cli::new(config).await?;
        cli.execute(&query).await?;
        return Ok(());
    }

    if let Some(file) = args.file {
        let mut cli = Cli::new(config).await?;
        cli.execute_file(&file).await?;
        return Ok(());
    }

    let mut repl = Repl::new(config).await?;
    repl.run().await?;

    Ok(())
}
