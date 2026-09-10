use std::{num::NonZeroU64, path::PathBuf};

use crate::config::ConfigPollingArgs;

pub struct ConfigPollingLoadTest {
    proxy_url: String,
    actors_file: PathBuf,
    requests_per_second: NonZeroU64,
}

impl ConfigPollingLoadTest {
    #[must_use]
    pub fn new(args: ConfigPollingArgs) -> Self {
        Self {
            proxy_url: args.proxy_url,
            actors_file: args.devices_file,
            requests_per_second: args.requests_per_second,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let _ = (self.proxy_url, self.actors_file, self.requests_per_second);
        tracing::warn!("config-polling load test is not implemented yet");
        Ok(())
    }
}
