use clap::Parser;
use mikudb_server::{Server, ServerConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "mikudb-server")]
#[command(author = "MikuDB Team")]
#[command(version)]
#[command(about = "MikuDB Server - High-performance document database for OpenEuler")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    #[arg(short, long, default_value_t = 3939)]
    port: u16,

    #[arg(short, long, env = "MIKUDB_CONFIG")]
    config: Option<PathBuf>,

    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    #[arg(long, default_value = "info")]
    log_level: String,

    #[arg(long)]
    daemon: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    mikudb_server::init_logging(&args.log_level);

    mikudb_core::print_banner();

    let config = if let Some(config_path) = &args.config {
        info!("Loading config from {:?}", config_path);
        ServerConfig::from_file(config_path)?
    } else {
        ServerConfig {
            bind: args.bind,
            port: args.port,
            data_dir: args.data_dir,
            ..Default::default()
        }
    };

    info!("Starting MikuDB server on {}:{}", config.bind, config.port);

    let server = Arc::new(Server::new(config).await?);

    tokio::select! {
        result = server.clone().run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(anyhow::anyhow!("{}", e));
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("MikuDB server stopped");
    Ok(())
}
