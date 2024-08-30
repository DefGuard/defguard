use std::{
    sync::{RwLock, RwLockReadGuard},
    time::Duration,
};

use anyhow::Result;
use base64::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use humantime::format_duration;
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use prost::Message;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::sleep;

use crate::{
    db::{DbPool, Settings},
    VERSION,
};

static LICENSE: RwLock<Option<License>> = RwLock::new(None);

pub fn set_cached_license(license: Option<License>) {
    *LICENSE
        .write()
        .expect("Failed to acquire lock on the license mutex.") = license;
}

pub fn get_cached_license() -> RwLockReadGuard<'static, Option<License>> {
    LICENSE
        .read()
        .expect("Failed to acquire lock on the license mutex.")
}

tonic::include_proto!("license");

// Mock license key
#[cfg(test)]
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mQENBGa0jtoBCAC63WkY0btyVzHI8JGVfIkFClNggcDgK+X/if5ndJtHKRXcW6DB
bRTBNCdUr7sDzCMEYWu8t400Yn/mrLKuubA3G6rp3Eo2nHnOicoZ6mfAdUQL862l
m9M8zpJtFodWR5G0nznyvabQi9kI1JT87DEIAdfLhN4eoMpgEm+jASSgFeT63oJ9
fLHofMZLwYZW/mqsnGxElmUsfnVWeseUSgmKBP4IgdtX4LsCx8XiOyQJww6bEUTj
ZBSqwwuRa1ybtsV3ihEKjDBmXQo5+J3fsadm/6m5PRJVk5rq9/LGVKIBG9m/x6Pn
xeYaLsjNyAwOSHH2KpeBLPVEfjsqWRt8fyAzABEBAAG0HEF1dG9nZW5lcmF0ZWQg
S2V5IDxkZWZndWFyZD6JAU4EEwEKADgWIQTyH9Rb8S5I78bRYzghGgZ+AdnRKwUC
ZrSO2gIbLwULCQgHAgYVCgkICwIEFgIDAQIeAQIXgAAKCRAhGgZ+AdnRKyzzCACW
oGBnAPHkCuvlnZjcYUAJVrjI/S02x4t3wFjaFOu+GQSjeB+AjDawF/S4D5ReQ8iq
D3dTvno3lk/F5HvqV/ZDU9WMmkDFzJoEwKbNIlWwQvvrTnoyy7lpKskNxwwsErEL
2+rW+lW/N5KNHFaUh2d5JhK08VRPfyl0WA8gqQ99Wnhq4rHF7ijKFm3im0RlzkMI
NTXxxee/9J0/Pzh+7zFZlMxnnjwiHlxJXpQFwh7+TS9C3IpChW3ipyPgp1DkzsNv
Xry1crUOhOyEozdKYh2H6tZEi3bjtGwpYkXJs/g3f6HPKjS8rDOMXw4Japb7LYtC
Aao60J8cOm8J96u1MsUK
=6cHp
-----END PGP PUBLIC KEY BLOCK-----
";

#[cfg(not(test))]
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mQGNBGbFl2QBDACxmjXHE5oHD8J2i7VpusbjrQGPd6IIzosy0AnES2Eli+O+WYK+
6I1KWTo/kapbA7KyBQxRrWqC8nP3B0hNhhNIkjdXB3UskKdrOaRRmOUUGYigSrXR
clC1rx+w0QU9vlBZ/dcgLhaKwQ7jY6w6alsic/7Gt2yA1226uMja1Da2PHjitZ/c
GFKq8f3tg6eG3I3czYX0FEAQ5fRxFuOKG+tSpThpV2rEmA48V7Tdeuf4pIbDA4Gy
LXbmMsDMt5nfTCcnQU2l0Ed+RW4f7WTX+Am/z6+hyisv8x0w2SjrKXQQL6CoO9xu
JkTcLYBg6g8tDPDSHPboLNnAPPlj3SAAhRnRZJEbaUEvRRRvEPgjgNBt1HlkPZgb
zUpF1gma/s2HwHROdSu4dEcgGppaLCkHz1/4j05ErrSRZ1p0CQXKQZtz7FQGxo0s
B1WTwsJL8P+ZMm2ZeaLtFGfPNAuIsDB+JhXslhL3yXxcNYpXw5x9yw3hIStFibfb
LXA6DUUOJW5r2I0AEQEAAbQdRGVmZ3VhcmQgPHNhbGVzQGRlZmd1YXJkLm5ldD6J
AdcEEwEKAEEWIQTIqIb4JsmVpy12RMLl6Wgl7mGgOgUCZsWXZAIbAwUJBaO9vAUL
CQgHAgIiAgYVCgkICwIEFgIDAQIeBwIXgAAKCRDl6Wgl7mGgOmz0DACxVkEDEmlq
L272Jydb9B0oZJzkQXf4S+GBZzyrWB7YBPVk/2sfGPmfJu9QGP/INkEu80OBtnBi
yN5qmxagESvyf7zkw/xls/AFJrwDamGm2w5/QWJQTdvxV85/0AX5eWOxfB5V79oG
NJ/CutFA8oKktx2OZOJmMAHP2ihn24nsxNNuZbJ4N/81UavjkGOzbAzar8yPOB0u
npy+DZCKoK8lRNZw+ebwxhzTL0zXKQlYVXNcuJzjAqkoF1g9aLRPCENnmrSfoRL8
5ChGvrh+fnvnCZTKeNumzSMvPGT2WpjhkP9GhR6Il+JodI1WgF4VGtFk7m1VjdYq
B4Kk5t5duwkolrHN0BHlw6VmcNvjCNFXcu8Q14JVnBMGRQarikx5CeIQ34chumYL
V608LjgziJHk2Z4LslZnGl+saPYwLozxIR16xN0wc5QFBwk/vnR3oTAZq1Nhtq1n
EK7PrFMXzNlxP2OD6yIkYgEdSX5/99nPes0i0pQf4kdVN713ydHyIta5AY0EZsWX
ZAEMAL6TXF2PHWVRUvY4jO/kqUdObBoiw+vNu0gyjiaId6bu7fJarNUprK/o+Wkd
mSqPQdIL011F8exCOKkQ9q+P0UGktl2zQNg5XYZjK4Ii+6qwdM3jS09ZRxhljJh2
yNb1TgOGrCPzsp/Ii+71ENndzB/y/K5JYtZTEoQZNfc+B3MKRY+UvZ53YWyLaD8J
VbkmdzAX7gNoAzGpmcsoe9dQj8Bl4Al2j/i4EBmCBenscjOIQERdLDKOoqsxgfe4
8+GxKXE4A/d4qRpMSw5bZPCsDKFu5fClUeN1SZUX1//daiB5gwE4NaoNVP4Odogw
0i+3bTwA+xTvVj+3XSb6doPOq5HtyMf0ELK6zcGDAH8pI53IqEMC/ABANN4ahLUg
d/yK6R28KrhLJJDZQfzooDEYu2JKvpsB76ox5ou5Cuga8zHC+FX0NYA+rKjsVO6O
Txidl7gW+mtgashBSTR0TrSHIxpwthBuKAY53t8vejHTryxpXxmK6+A2P+yDZfDj
eEKJtQARAQABiQG8BBgBCgAmFiEEyKiG+CbJlactdkTC5eloJe5hoDoFAmbFl2QC
GwwFCQWjvbwACgkQ5eloJe5hoDrIRwv+PekGfNtDDR9TfWX2rCexzE1/JOMaA1dO
QXLFPpIwtjEsv6yuIMu8zqUIoI0NV72NU89IxKyngJxMQuVhD1LDLmOpBWe/Jyr7
wvrFlAqpVBiGckjfSiAUVjWjQp9AFY+n5PEGJ/zW6VfshTD3PQ7mZrk7i6rfyueo
9iRKZkt7S5DT3F/srJum7ev/f4z8bDDvlAO7VqCMEXX3t3/SbGZPETYW7odnncWM
Lbcwv7rP7GXGJI01g1D3oDtqnkcYDZSznmyI7Ihus20Ak/RicZyLnGLr/G15T1LL
l3murdotb0bzhlQ8spuMEfYnnv0E0klY3f9YG5qm+ey1Yg959+pH/W3xsWq0rLtW
6/Mj2mXHreWQpT3KRwabO+2DkITRabEtSdvOfEX9j0o8kpQRC24x9Pg3Tk6bo+ww
OtCZRnxvKx9sqxOQrg4Lkh9OrAeziPQcMWROJ06+GveMgHtxghCJVTh7pCr+9Rqp
IQyvDB2pcQYgS91DqeDU1BRosIlCkpeh
=vSch
-----END PGP PUBLIC KEY BLOCK-----
";

#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("Provided license is invalid: {0}")]
    InvalidLicense(String),
    #[error("Provided signature does not match the license")]
    SignatureMismatch,
    #[error("Provided signature is invalid")]
    InvalidSignature,
    #[error("Database error")]
    DbError(#[from] SqlxError),
    #[error("License decoding error: {0}")]
    DecodeError(String),
    #[error("License is expired and has reached its maximum overdue time, please contact sales<at>defguard.net")]
    LicenseExpired,
    #[error("License not found")]
    LicenseNotFound,
    #[error("License server error: {0}")]
    LicenseServerError(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshRequestResponse {
    key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct License {
    pub customer_id: String,
    pub subscription: bool,
    pub valid_until: Option<DateTime<Utc>>,
}

impl License {
    #[must_use]
    pub fn new(
        customer_id: String,
        subscription: bool,
        valid_until: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            customer_id,
            subscription,
            valid_until,
        }
    }

    fn decode(bytes: &[u8]) -> Result<Vec<u8>, LicenseError> {
        let bytes = BASE64_STANDARD.decode(bytes).map_err(|_| {
            LicenseError::DecodeError(
                "Failed to decode the license key, check if the provided key is correct."
                    .to_string(),
            )
        })?;
        Ok(bytes)
    }

    fn verify_signature(data: &[u8], signature: &[u8]) -> Result<(), LicenseError> {
        // A hardcoded public key should be always valid, so we can unwrap here
        let (public_key, _headers_public) = SignedPublicKey::from_string(PUBLIC_KEY).unwrap();
        let sig = StandaloneSignature::from_bytes(signature)
            .map_err(|_| LicenseError::InvalidSignature)?;
        sig.verify(&public_key, data)
            .map_err(|_| LicenseError::SignatureMismatch)
    }

    /// Deserialize the license object from a base64 encoded string.
    /// Also verifies the signature of the license
    pub fn from_base64(key: &str) -> Result<License, LicenseError> {
        debug!("Decoding the license key from a provided base64 string...");
        let bytes = key.as_bytes();
        let decoded = Self::decode(bytes)?;
        let slice: &[u8] = &decoded;
        debug!("Decoded the license key, deserializing the license object...");

        let license_key = LicenseKey::decode(slice).map_err(|_| {
            LicenseError::DecodeError(
                "The license key is malformed, check if the provided key is correct.".to_string(),
            )
        })?;

        let metadata_bytes: &[u8] = &license_key.metadata;
        let signature_bytes: &[u8] = &license_key.signature;

        debug!("Deserialized the license object, verifying the license signature...");

        match Self::verify_signature(metadata_bytes, signature_bytes) {
            Ok(_) => {
                info!("Successfully decoded the license and validated the license signature");
                let metadata = LicenseMetadata::decode(metadata_bytes).map_err(|_| {
                    LicenseError::DecodeError("Failed to decode the license metadata".to_string())
                })?;

                let valid_until = match metadata.valid_until {
                    Some(until) => DateTime::from_timestamp(until, 0),
                    None => None,
                };

                let license =
                    License::new(metadata.customer_id, metadata.subscription, valid_until);

                if license.requires_renewal() {
                    if license.is_max_overdue() {
                        warn!("The provided license has expired and reached its maximum overdue time, please contact sales<at>defguard.net");
                    } else {
                        warn!("The provided license is about to expire and requires a renewal. An automatic renewal process will attempt to renew the license soon. Alternatively, automatic renewal attempt will be also performed at the next defguard start.");
                    }
                }

                if !license.subscription && license.is_expired() {
                    warn!("The provided license is not a subscription and has expired, please contact sales<at>defguard.net");
                }

                Ok(license)
            }
            Err(_) => Err(LicenseError::SignatureMismatch),
        }
    }

    /// Get the key from the database
    async fn get_key(pool: &DbPool) -> Result<Option<String>, LicenseError> {
        let settings = Settings::get_settings(pool).await?;
        match settings.license {
            Some(key) => {
                if key.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(key))
                }
            }
            None => Ok(None),
        }
    }

    /// Create the license object based on the license key stored in the database.
    /// Automatically decodes and deserializes the keys and verifies the signature.
    pub async fn load(pool: &DbPool) -> Result<Option<License>, LicenseError> {
        match Self::get_key(pool).await? {
            Some(key) => Ok(Some(Self::from_base64(&key)?)),
            None => {
                debug!("No license key found in the database");
                Ok(None)
            }
        }
    }

    /// Try to load the license from the database, if the license requires a renewal, try to renew it.
    /// If the renewal fails, it will return the old license for the renewal service to renew it later.
    pub async fn load_or_renew(pool: &DbPool) -> Result<Option<License>, LicenseError> {
        match Self::load(pool).await? {
            Some(license) => {
                if license.requires_renewal() {
                    if !license.is_max_overdue() {
                        info!("License requires renewal, trying to renew it...");
                        match renew_license(pool).await {
                            Ok(new_key) => {
                                let new_license = License::from_base64(&new_key)?;
                                save_license_key(pool, &new_key).await?;
                                info!("Successfully renewed and loaded the license, new license key saved to the database");
                                Ok(Some(new_license))
                            }
                            Err(err) => {
                                error!("Failed to renew the license: {err}");
                                Ok(Some(license))
                            }
                        }
                    } else {
                        Err(LicenseError::LicenseExpired)
                    }
                } else {
                    info!("Successfully loaded the license from the database.");
                    Ok(Some(license))
                }
            }
            None => Ok(None),
        }
    }

    /// Checks whether the license is past its expiry date (`valid_until` field)
    ///
    /// NOTE: license should be considered valid for an additional period of `MAX_OVERDUE_TIME`.
    /// If you want to check if the license reached this point, use `is_max_overdue` instead.
    pub fn is_expired(&self) -> bool {
        match self.valid_until {
            Some(time) => time < Utc::now(),
            None => false,
        }
    }

    /// Checks how much time has left until the `valid_until` time.
    pub fn time_left(&self) -> Option<TimeDelta> {
        self.valid_until.map(|time| time - Utc::now())
    }

    /// Gets the time the license is past its expiry date.
    /// If the license doesn't have a `valid_until` field, it will return 0.
    pub fn time_overdue(&self) -> TimeDelta {
        match self.valid_until {
            Some(time) => {
                let delta = Utc::now() - time;
                if delta <= TimeDelta::zero() {
                    TimeDelta::zero()
                } else {
                    delta
                }
            }
            None => TimeDelta::zero(),
        }
    }

    /// Checks whether we should try to renew the license.
    pub fn requires_renewal(&self) -> bool {
        if self.subscription {
            if let Some(remaining) = self.time_left() {
                remaining <= RENEWAL_TIME
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Checks if the license has reached its maximum overdue time.
    pub fn is_max_overdue(&self) -> bool {
        if !self.subscription {
            // Non-subscription licenses are considered expired immediately, no grace period is required
            self.is_expired()
        } else {
            self.time_overdue() > MAX_OVERDUE_TIME
        }
    }
}

/// Exchange the currently stored key for a new one from the license server.
///
/// Doesn't update the cached license, nor does it save the new key in the database.
async fn renew_license(db_pool: &DbPool) -> Result<String, LicenseError> {
    debug!("Exchanging license for a new one...");
    let old_license_key = match Settings::get_settings(db_pool).await?.license {
        Some(key) => key,
        None => return Err(LicenseError::LicenseNotFound),
    };

    let client = reqwest::Client::new();

    let request_body = RefreshRequestResponse {
        key: old_license_key,
    };

    // FIXME: this should be a hardcoded IP, make sure to add appropriate host headers
    const LICENSE_SERVER_URL: &str = "https://update-service-dev.teonite.net/api/license/renew";

    let new_license_key = match client
        .post(LICENSE_SERVER_URL)
        .json(&request_body)
        .header(reqwest::header::USER_AGENT, format!("DefGuard/{VERSION}"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => {
                let response: RefreshRequestResponse = response.json().await.map_err(|err| {
                    error!("Failed to parse the response from the license server while trying to renew the license: {err:?}");
                    LicenseError::LicenseServerError(err.to_string())
                })?;
                response.key
            }
            status => {
                let status_message = response.text().await.unwrap_or_default();
                let message = format!(
                    "Failed to renew the license, the license server returned a status code {status} with error: {status_message}"
                );
                return Err(LicenseError::LicenseServerError(message));
            }
        },
        Err(err) => {
            return Err(LicenseError::LicenseServerError(err.to_string()));
        }
    };

    info!("Successfully exchanged the license for a new one");

    Ok(new_license_key)
}

/// Helper function used to check if the cached license should be considered valid.
/// As the license is often passed around in the form of `Option<License>`, this function takes care
/// of the whole logic related to checking whether the license is even present in the first place.
///
/// This function checks the following two things:
/// 1. Does the cached license exist
/// 2. Does the cached license is past its maximum expiry date
pub fn validate_license(license: Option<&License>) -> Result<(), LicenseError> {
    debug!("Validating if the license is present and not expired...");
    match license {
        Some(license) => {
            if license.is_max_overdue() {
                return Err(LicenseError::LicenseExpired);
            }
            Ok(())
        }
        None => Err(LicenseError::LicenseNotFound),
    }
}

/// Helper function to save the license key string in the database
async fn save_license_key(pool: &DbPool, key: &str) -> Result<(), LicenseError> {
    debug!("Saving the license key to the database...");
    let mut settings = Settings::get_settings(pool).await?;
    settings.license = Some(key.to_string());
    settings.save(pool).await?;
    info!("Successfully saved the license key to the database.");
    Ok(())
}

/// Helper function to update the cached license mutex. The mutex is used mainly in the appstate.
pub fn update_cached_license(key: Option<&str>) -> Result<(), LicenseError> {
    debug!("Updating the cached license information with the provided key...");
    let license = if let Some(key) = key {
        // Handle the Some("") case
        if key.is_empty() {
            debug!("The new license key is empty, clearing the cached license");
            None
        } else {
            debug!("A new license key has been provided, decoding and validating it...");
            Some(License::from_base64(key)?)
        }
    } else {
        None
    };
    set_cached_license(license);
    info!("Successfully updated the cached license information.");
    Ok(())
}

/// Amount of time before the license expiry date we should start the renewal attempts.
const RENEWAL_TIME: TimeDelta = TimeDelta::hours(24);

/// Maximum amount of time a license can be over its expiry date.
const MAX_OVERDUE_TIME: TimeDelta = TimeDelta::days(14);

/// Periodic license check task
const CHECK_PERIOD: Duration = Duration::from_secs(12 * 60 * 60);

/// Periodic license check task for the case when no license is present
const CHECK_PERIOD_NO_LICENSE: Duration = Duration::from_secs(24 * 60 * 60);

/// Periodic license check task for the case when the license is about to expire
const CHECK_PERIOD_RENEWAL_WINDOW: Duration = Duration::from_secs(60 * 60);

pub async fn run_periodic_license_check(pool: DbPool) -> Result<(), LicenseError> {
    let mut check_period: Duration = CHECK_PERIOD;
    info!(
        "Starting periodic license renewal check every {}",
        format_duration(check_period)
    );
    loop {
        debug!("Checking the license status...");
        // Check if the license is present in the mutex, if not skip the check
        if get_cached_license().is_none() {
            debug!("No license found, skipping license check");
            sleep(CHECK_PERIOD_NO_LICENSE).await;
            continue;
        }

        // Check if the license requires renewal, uses the cached value to be more efficient
        // The block here is to avoid holding the lock through awaits
        //
        // Multiple locks here may cause a race condition if the user decides to update the license key
        // while the renewal is in progress. However this seems like a rare case and shouldn't be very problematic.
        let requires_renewal = {
            let license = get_cached_license();
            debug!("Checking if the license {license:?} requires a renewal...");

            match &*license {
                Some(license) => {
                    if license.requires_renewal() {
                        // check if we are pass the maximum expiration date, after which we don't
                        // want to try to renew the license anymore
                        if !license.is_max_overdue() {
                            debug!("License requires renewal, as it is about to expire and is not past the maximum overdue time");
                            true
                        } else {
                            check_period = CHECK_PERIOD;
                            warn!("Your license has expired and reached its maximum overdue date, please contact sales at sales<at>defguard.net");
                            debug!("Changing check period to {}", format_duration(check_period));
                            false
                        }
                    } else {
                        // This if is only for logging purposes, to provide more detailed information
                        if license.subscription {
                            debug!("License doesn't need to be renewed yet, skipping renewal check")
                        } else {
                            debug!("License is not a subscription, skipping renewal check")
                        }
                        false
                    }
                }
                None => {
                    debug!("No license found, skipping license check");
                    false
                }
            }
        };

        if requires_renewal {
            info!("License requires renewal, renewing license...");
            check_period = CHECK_PERIOD_RENEWAL_WINDOW;
            debug!("Changing check period to {}", format_duration(check_period));
            match renew_license(&pool).await {
                Ok(new_license_key) => match save_license_key(&pool, &new_license_key).await {
                    Ok(_) => {
                        update_cached_license(Some(&new_license_key))?;
                        check_period = CHECK_PERIOD;
                        debug!("Changing check period to {}", format_duration(check_period));
                        info!("Successfully renewed the license");
                    }
                    Err(err) => {
                        error!("Couldn't save the newly fetched license key to the database, error: {}", err);
                    }
                },
                Err(err) => {
                    warn!(
                        "Failed to renew the license: {err}. Retrying in {} seconds",
                        format_duration(check_period)
                    );
                }
            }
        }

        sleep(check_period).await;
    }
}

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_license() {
        let license = "CigKIDVhMGRhZDRiOWNmZTRiNzZiYjkzYmI1Y2Q5MGM2ZjdjGLL+lrYGErYCiQEzBAABCgAdFiEE8h/UW/EuSO/G0WM4IRoGfgHZ0SsFAmbFvzUACgkQIRoGfgHZ0SuNQggAioLovxAyrgAn+LPO42QIlVHYG8oTs3jnpM0BMx3cXbfy7M0ECsC10HpzIkundems7SgYO/+iJfMMe4mj3kiA+uwacCmPW6VWTIVEIpX2jqRpv7DcDnUSeAszySZl6KhQS+35IPC0Gs2yQNU4/mDsa4VUv9DiL8s7rMM89fe4QmtjVRpFQVgGLm4IM+mRIXTySB2RwmVzw8+YE4z+w4emLxaKWjw4Q7CQxykkPNGlBj224jozs/Biw9eDYCbJOT/5KXNqZ2peht59n6RMVc0SNKE26E8hDmJ61M0Tzj57wQ6nZ3yh6KGyTdCIc9Y9wcrHwZ1Yw1tdh8j/fULUyPtNyA==";
        let license = License::from_base64(license).unwrap();
        assert_eq!(license.customer_id, "5a0dad4b9cfe4b76bb93bb5cd90c6f7c");
        assert!(!license.subscription);
        assert_eq!(
            license.valid_until.unwrap(),
            Utc.with_ymd_and_hms(2024, 8, 21, 10, 19, 30).unwrap()
        );

        assert!(license.is_expired());
    }

    #[test]
    fn test_new_license() {
        // This key has an additional test_field in the metadata that doesn't exist in the proto definition
        // It should still be able to decode the license correctly
        let license = "CjIKIDVhMGRhZDRiOWNmZTRiNzZiYjkzYmI1Y2Q5MGM2ZjdjGMv0lrYGIggxMjM0NTY3OBK2AokBMwQAAQoAHRYhBPIf1FvxLkjvxtFjOCEaBn4B2dErBQJmxbpSAAoJECEaBn4B2dEru6sH/0FBWgj8Nl1n/hwx1CdwrmKkKOCRpTf244wS07EcwQDr/A5TA011Y4PFJBSFfoIlyuGFHh20KoczFVUPfyiIGkqMMGOe8BH0Pbst6n5hd1S67m5fKgNV+NdaWg1aJfMdbGdworpZWTnsHnsTnER+fhoC/CohPtTshTdBZX0wmyfAWKQW3HM0YcE73+KFvGMzTMyin/bOrjr7bW0d5yoQLaEIpAASTlb6DaX5avyTFitXLf77cMjRu4wysnlPfwIpSqQI+ESHNh+OepOUqxmox+U9hGVtvlIJhvBOLgJ/Kmldc1Kj7uZaldLhWDG5e7+dVdnhbwfuoUsgS9jmpAmeWsg=";
        let license = License::from_base64(license).unwrap();

        assert_eq!(license.customer_id, "5a0dad4b9cfe4b76bb93bb5cd90c6f7c");
        assert!(!license.subscription);
        assert_eq!(
            license.valid_until.unwrap(),
            Utc.with_ymd_and_hms(2024, 8, 21, 9, 58, 35).unwrap()
        );
    }
}
