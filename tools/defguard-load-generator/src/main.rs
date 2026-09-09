use tracing_subscriber::EnvFilter;
use clap::Parser;

mod config;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = config::Config::parse();
    tracing::info!(?config, "running with config");
}
