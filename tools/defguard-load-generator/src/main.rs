use clap::Parser;
use tracing_subscriber::EnvFilter;

mod config;
mod export;
mod loadtest;
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
        config::Command::Export(config::ExportArgs {
            command: config::ExportCommand::ConfigPolling(args),
        }) => export::export_config_polling(args).await?,
        config::Command::Test(config::TestArgs { command }) => match command {
            config::TestCommand::ConfigPolling(args) => {
                loadtest::ConfigPollingLoadTest::new(args).run().await?
            }
        },
    }

    Ok(())
}
