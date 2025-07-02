use std::env;

use chrono::NaiveDate;
use semver::Version;

use crate::global_value;

const PRODUCT_NAME: &str = "Defguard";
const UPDATES_URL: &str = "https://pkgs.defguard.net/api/update/check";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize, Debug, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct Update {
    version: String,
    release_date: NaiveDate,
    release_notes_url: String,
    update_url: String,
    critical: bool,
    notes: String,
}

global_value!(NEW_UPDATE, Option<Update>, None, set_update, get_update);

async fn fetch_update() -> Result<Update, anyhow::Error> {
    let body = serde_json::json!({
        "product": PRODUCT_NAME,
        "client_version": VERSION,
        "operating_system": env::consts::OS,
    });
    let response = reqwest::Client::new()
        .post(UPDATES_URL)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;
    Ok(response.json::<Update>().await?)
}

pub(crate) async fn do_new_version_check() -> Result<(), anyhow::Error> {
    debug!("Checking for new version of Defguard ...");
    let update = fetch_update().await?;
    let current_version = Version::parse(VERSION)?;
    let new_version = Version::parse(&update.version)?;
    if new_version > current_version {
        if update.critical {
            warn!("There is a new critical Defguard update available: {} (Released on {}). It's recommended to update as soon as possible.",
                  update.version, update.release_date);
        } else {
            info!(
                "There is a new Defguard version available: {} (Released on {})",
                update.version, update.release_date
            );
        }
        set_update(Some(update));
    } else {
        debug!("New version check done. You are using the latest version of Defguard.");
    }
    Ok(())
}
