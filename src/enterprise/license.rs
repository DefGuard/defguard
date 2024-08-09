use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};
use tokio::time::sleep;
// use rsa::{pkcs8::FromPublicKey, PaddingScheme, PublicKey, RsaPublicKey};

use crate::{
    db::{DbPool, Settings},
    server_config,
};
use base64::prelude::*;
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use prost::Message;
use sqlx::error::Error as SqlxError;
use thiserror::Error;

tonic::include_proto!("license");

#[allow(dead_code)]
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

#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("Provided license is invalid: {0}")]
    InvalidLicense(String),
    #[error("Provided signature is does not match the license")]
    SignatureMismatch,
    #[error("Provided signature is invalid")]
    InvalidSignature,
    #[error("Database error")]
    DbError(#[from] SqlxError),
    #[error("License decoding error: {0}")]
    DecodeError(String),
    #[error("License is expired and has reached its maximum overdue time, please contact sales")]
    LicenseExpired,
    #[error("License is not found")]
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
                "Failed to decode the Base64 license key, check if the provided key is correct."
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

    /// Deserialize the license object from a base64 encoded string
    /// Also verifies the signature of the license
    pub fn from_base64(key: &str) -> Result<License, LicenseError> {
        let bytes = key.as_bytes();
        let decoded = Self::decode(bytes)?;
        let slice: &[u8] = &decoded;

        let license_key = LicenseKey::decode(slice).map_err(|_| {
            LicenseError::DecodeError(
                "Failed to decode the binary license key, check if the provided key is correct."
                    .to_string(),
            )
        })?;
        let metadata = license_key.metadata.ok_or(LicenseError::InvalidLicense(
            "License metadata is missing from the license key".to_string(),
        ))?;
        let signature = license_key.signature.ok_or(LicenseError::InvalidLicense(
            "License signature is missing from the license key".to_string(),
        ))?;
        let metadata_bytes = metadata.encode_to_vec();

        match Self::verify_signature(&metadata_bytes, &signature.signature) {
            Ok(_) => {
                info!("Successfully validated license signature");
                let valid_until = match metadata.valid_until {
                    Some(until) => DateTime::from_timestamp(until, 0),
                    None => None,
                };

                Ok(License::new(
                    metadata.customer_id,
                    metadata.subscription,
                    valid_until,
                ))
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

    pub async fn load(pool: &DbPool) -> Result<Option<License>, LicenseError> {
        match Self::get_key(pool).await? {
            Some(key) => Ok(Some(Self::from_base64(&key)?)),
            None => {
                info!("No license key found in the database");
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
                        match renew_license(pool).await {
                            Ok(new_key) => {
                                let new_license = License::from_base64(&new_key)?;
                                save_license_key(pool, &new_key).await?;
                                Ok(Some(new_license))
                            }
                            Err(err) => {
                                error!("Failed to renew the license: {}", err);
                                Ok(Some(license))
                            }
                        }
                    } else {
                        Err(LicenseError::LicenseExpired)
                    }
                } else {
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
        self.time_overdue() > MAX_OVERDUE_TIME
    }
}

/// Exchange the currently stored key for a new one from the license server.
///
/// Doesn't update the cached license, nor does it save the new key in the database.
async fn renew_license(db_pool: &DbPool) -> Result<String, LicenseError> {
    info!("Exchanging license for a new one...");
    let old_license_key = match Settings::get_settings(db_pool).await?.license {
        Some(key) => key,
        None => return Err(LicenseError::LicenseNotFound),
    };

    let client = reqwest::Client::new();

    let request_body = RefreshRequestResponse {
        key: old_license_key,
    };

    let license_server_url = &server_config().license_server_url;

    let new_license_key = match client
        .post(license_server_url)
        .json(&request_body)
        .send()
        .await
    {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => {
                let response: RefreshRequestResponse = response.json().await.map_err(|err| {
                    error!("Failed to parse the response from the license server while trying to renew the license: {:?}", err);
                    LicenseError::LicenseServerError(err.to_string())
                })?;
                response.key
            }
            status => {
                let status_message = response.text().await.unwrap_or_else(|_| "".to_string());
                let message = format!(
                    "Failed to renew the license, the license server returned a status code {} with error: {}",
                    status, status_message
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
    let mut settings = Settings::get_settings(pool).await?;
    settings.license = Some(key.to_string());
    settings.save(pool).await?;
    Ok(())
}

/// Helper function to update the cached license mutex. The mutex is used mainly in the appstate.
pub fn update_cached_license(
    key: Option<&str>,
    license_mutex: Arc<Mutex<Option<License>>>,
) -> Result<(), LicenseError> {
    let license = if let Some(key) = key {
        // Handle the Some("") case
        if key.is_empty() {
            None
        } else {
            Some(License::from_base64(key)?)
        }
    } else {
        None
    };
    *license_mutex
        .lock()
        .expect("Failed to acquire lock on the license mutex.") = license;
    Ok(())
}

/// Amount of time before the license expiry date we should start the renewal attempts.
const RENEWAL_TIME: TimeDelta = TimeDelta::hours(24);

/// Maximum amount of time a license can be over its expiry date.
const MAX_OVERDUE_TIME: TimeDelta = TimeDelta::hours(24);

pub async fn run_periodic_license_check(
    pool: DbPool,
    license_mutex: Arc<Mutex<Option<License>>>,
) -> Result<(), LicenseError> {
    let mut check_period = server_config().license_check_period;
    info!("Starting periodic license check every {:?}", check_period);
    loop {
        // Check if the license is present in the mutex, if not skip the check
        if license_mutex
            .lock()
            .expect("Failed to acquire lock on the license mutex.")
            .is_none()
        {
            info!("No license found, skipping license check");
            sleep(*server_config().license_check_period_no_license).await;
            continue;
        }

        // Check if the license requires renewal, uses the cached value to be more efficient
        // The block here is to avoid holding the lock through awaits
        let requires_renewal = {
            let license = license_mutex
                .lock()
                .expect("Failed to acquire lock on the license mutex.");
            info!(
                "Checking if the license {:?} requires a renewal...",
                license
            );

            match &*license {
                Some(license) => {
                    if license.requires_renewal() {
                        if !license.is_max_overdue() {
                            true
                        } else {
                            check_period = server_config().license_check_period;
                            warn!("Your license has expired and reached its maximum overdue date, please contact sales at sales<at>defguard.net");
                            info!("Changing check period to {}", check_period);
                            false
                        }
                    } else {
                        false
                    }
                }
                None => false,
            }
        };

        if requires_renewal {
            info!("License requires renewal, renewing license...");
            check_period = server_config().license_check_period_renewal_window;
            info!("Changing check period to {}", check_period);
            match renew_license(&pool).await {
                Ok(new_license_key) => match save_license_key(&pool, &new_license_key).await {
                    Ok(_) => {
                        update_cached_license(Some(&new_license_key), license_mutex.clone())?;
                        check_period = server_config().license_check_period;
                        info!("Changing check period to {} seconds", check_period);
                    }
                    Err(err) => {
                        error!("Couldn't save the newly fetched license key to the database, error: {}", err);
                    }
                },
                Err(err) => {
                    error!("Failed to renew the license: {}", err);
                }
            }
        } else {
            info!("License isn't eligible for renewal, skipping...");
        }

        sleep(*check_period).await;
    }
}