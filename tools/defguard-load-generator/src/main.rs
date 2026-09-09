use clap::Parser;
use tracing_subscriber::EnvFilter;

mod config;
mod seed;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = config::Config::parse();

    match config.command {
        config::Command::Seed(args) => seed::run(args).await?,
        config::Command::Test(_) => tracing::warn!("test scenarios are not implemented yet"),
    }

    Ok(())
}
