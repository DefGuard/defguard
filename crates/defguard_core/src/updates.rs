use std::{env, time::Duration};

use chrono::NaiveDate;
use defguard_common::{REPORTED_VERSION, global_value};
use defguard_version::is_version_lower;
use semver::Version;

const PRODUCT_NAME: &str = "Defguard";
const UPDATES_URL: &str = "https://pkgs.defguard.net/api/update/check";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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
        "client_version": REPORTED_VERSION,
        "operating_system": env::consts::OS,
    });
    let response = reqwest::Client::new()
        .post(UPDATES_URL)
        .json(&body)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?;
    Ok(response.json::<Update>().await?)
}

fn is_newer_update_available(current_version: &Version, new_version: &Version) -> bool {
    is_version_lower(current_version, new_version)
}

pub(crate) async fn do_new_version_check() -> Result<(), anyhow::Error> {
    debug!("Checking for new version of Defguard.");
    let update = fetch_update().await?;
    let current_version = Version::parse(REPORTED_VERSION)?;
    let new_version = Version::parse(&update.version)?;
    if is_newer_update_available(&current_version, &new_version) {
        if update.critical {
            warn!(
                "There is a new critical Defguard update available: {} (Released on {}). It's \
                recommended to update as soon as possible.",
                update.version, update.release_date
            );
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

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::is_newer_update_available;

    #[test]
    fn ignores_prerelease_suffixes_when_comparing_update_versions() {
        let cases = [
            ("2.0.0-beta1", "2.0.0", false),
            ("2.0.0", "2.0.0-beta1", false),
            ("2.0.0-beta1", "2.0.0-rc1", false),
            ("2.0.0-beta1", "2.0.1", true),
            ("2.0.0-beta1", "2.1.0-alpha1", true),
            ("2.0.1", "2.0.0-rc1", false),
        ];

        for (current, new, expected) in cases {
            let current = Version::parse(current).expect("valid current version");
            let new = Version::parse(new).expect("valid new version");
            assert_eq!(
                is_newer_update_available(&current, &new),
                expected,
                "current={current}, new={new}"
            );
        }
    }
}
