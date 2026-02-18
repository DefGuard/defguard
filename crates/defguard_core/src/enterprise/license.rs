use std::{fmt, time::Duration};

use anyhow::Result;
use base64::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use defguard_common::{
    VERSION,
    config::server_config,
    db::models::{Settings, settings::update_current_settings},
    global_value,
};
use humantime::format_duration;
use pgp::{
    composed::{Deserializable, DetachedSignature, SignedPublicKey},
    types::KeyDetails,
};
use prost::Message;
use sqlx::{PgPool, error::Error as SqlxError};
use thiserror::Error;
use tokio::time::sleep;

use super::limits::Counts;
use crate::grpc::proto::enterprise::license::{
    LicenseKey, LicenseLimits, LicenseMetadata, LicenseTier as LicenseTierProto,
};

const LICENSE_SERVER_URL: &str = "https://pkgs.defguard.net/api/license/renew";

global_value!(
    LICENSE,
    Option<License>,
    None,
    set_cached_license,
    get_cached_license
);

#[cfg(not(test))]
pub(crate) const PUBLIC_KEY: &[u8] = include_bytes!("public_key.asc");

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
    #[error(
        "License is expired and has reached its maximum overdue time, please contact sales<at>defguard.net"
    )]
    LicenseExpired,
    #[error("License not found")]
    LicenseNotFound,
    #[error("License server error: {0}")]
    LicenseServerError(String),
    #[error(
        "License limits exceeded. To upgrade your license please contact sales<at>defguard.net"
    )]
    LicenseLimitsExceeded,
    #[error("License tier is lower than required minimum")]
    LicenseTierTooLow,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshRequestResponse {
    key: String,
}

/// Represents license tiers
///
/// Variant order must be maintained to go from lowest (first) to highest (last) tier
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum LicenseTier {
    Business, // this corresponds to both Team & Business level in our current pricing structure
    Enterprise,
}

impl fmt::Display for LicenseTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Business => "Business",

            Self::Enterprise => "Enterprise",
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct License {
    pub customer_id: String,
    pub subscription: bool,
    pub valid_until: Option<DateTime<Utc>>,
    pub limits: Option<LicenseLimits>,
    pub version_date_limit: Option<DateTime<Utc>>,
    pub tier: LicenseTier,
}

impl License {
    #[must_use]
    pub fn new(
        customer_id: String,
        subscription: bool,
        valid_until: Option<DateTime<Utc>>,
        limits: Option<LicenseLimits>,
        version_date_limit: Option<DateTime<Utc>>,
        tier: LicenseTier,
    ) -> Self {
        Self {
            customer_id,
            subscription,
            valid_until,
            limits,
            version_date_limit,
            tier,
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
        let sig =
            DetachedSignature::from_bytes(signature).map_err(|_| LicenseError::InvalidSignature)?;
        let public_key =
            SignedPublicKey::from_bytes(PUBLIC_KEY).expect("Failed to parse the public key");

        // If the public key has subkeys, extract the signing key from them
        // Otherwise, use the primary key
        if public_key.public_subkeys.is_empty() {
            debug!(
                "Using the public key's primary key {:?} to verify the signature...",
                public_key.legacy_key_id()
            );
            sig.verify(&public_key, data)
                .map_err(|_| LicenseError::SignatureMismatch)
        } else {
            let signing_key =
                public_key
                    .public_subkeys
                    .first()
                    .ok_or(LicenseError::LicenseServerError(
                        "Failed to find a signing key in the provided public key".to_string(),
                    ))?;
            debug!(
                "Using the public key's subkey {:?} to verify the signature...",
                signing_key.legacy_key_id()
            );
            sig.verify(&signing_key, data)
                .map_err(|_| LicenseError::SignatureMismatch)
        }
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
            Ok(()) => {
                info!("Successfully decoded the license and validated the license signature");
                let metadata = LicenseMetadata::decode(metadata_bytes).map_err(|_| {
                    LicenseError::DecodeError("Failed to decode the license metadata".to_string())
                })?;

                let valid_until = match metadata.valid_until {
                    Some(until) => DateTime::from_timestamp(until, 0),
                    None => None,
                };

                let version_date_limit = match metadata.version_date_limit {
                    Some(date) => DateTime::from_timestamp(date, 0),
                    None => None,
                };

                let license_tier = match LicenseTierProto::try_from(metadata.tier) {
                    Ok(LicenseTierProto::Enterprise) => LicenseTier::Enterprise,
                    // fall back to Business tier for legacy licenses
                    Ok(LicenseTierProto::Business | LicenseTierProto::Unspecified) => {
                        LicenseTier::Business
                    }
                    Err(err) => {
                        error!("Failed to read license tier from license metadata: {err}");
                        return Err(LicenseError::DecodeError(
                            "Failed to decode license tier metadata".into(),
                        ));
                    }
                };

                let license = License::new(
                    metadata.customer_id,
                    metadata.subscription,
                    valid_until,
                    metadata.limits,
                    version_date_limit,
                    license_tier,
                );

                if license.requires_renewal() {
                    if license.is_max_overdue() {
                        warn!(
                            "The provided license has expired and reached its maximum overdue time, please contact sales<at>defguard.net"
                        );
                    } else {
                        warn!(
                            "The provided license is about to expire and requires a renewal. An automatic renewal process will attempt to renew the license soon. Alternatively, automatic renewal attempt will be also performed at the next defguard start."
                        );
                    }
                }

                if !license.subscription && license.is_expired() {
                    warn!(
                        "The provided license is not a subscription and has expired, please contact sales<at>defguard.net"
                    );
                }

                Ok(license)
            }
            Err(_) => Err(LicenseError::SignatureMismatch),
        }
    }

    /// Get the key from the database
    fn get_key() -> Option<String> {
        let settings = Settings::get_current_settings();
        settings.license.filter(|key| !key.is_empty())
    }

    /// Create the license object based on the license key stored in the database.
    /// Automatically decodes and deserializes the keys and verifies the signature.
    pub fn load() -> Result<Option<License>, LicenseError> {
        if let Some(key) = Self::get_key() {
            Ok(Some(Self::from_base64(&key)?))
        } else {
            debug!("No license key found in the database");
            Ok(None)
        }
    }

    /// Try to load the license from the database, if the license requires a renewal, try to renew it.
    /// If the renewal fails, it will return the old license for the renewal service to renew it later.
    pub async fn load_or_renew(pool: &PgPool) -> Result<Option<License>, LicenseError> {
        match Self::load()? {
            Some(license) => {
                if license.requires_renewal() {
                    if license.is_max_overdue() {
                        Err(LicenseError::LicenseExpired)
                    } else {
                        info!("License requires renewal, trying to renew it...");
                        match renew_license().await {
                            Ok(new_key) => {
                                let new_license = License::from_base64(&new_key)?;
                                save_license_key(pool, &new_key).await?;
                                info!(
                                    "Successfully renewed and loaded the license, new license key saved to the database"
                                );
                                Ok(Some(new_license))
                            }
                            Err(err) => {
                                error!("Failed to renew the license: {err}");
                                Ok(Some(license))
                            }
                        }
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
    #[must_use]
    pub fn is_expired(&self) -> bool {
        match self.valid_until {
            Some(time) => time < Utc::now(),
            None => false,
        }
    }

    /// Checks how much time has left until the `valid_until` time.
    #[must_use]
    pub fn time_left(&self) -> Option<TimeDelta> {
        self.valid_until.map(|time| time - Utc::now())
    }

    /// Gets the time the license is past its expiry date.
    /// If the license doesn't have a `valid_until` field, it will return 0.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn is_max_overdue(&self) -> bool {
        if self.subscription {
            self.time_overdue() > MAX_OVERDUE_TIME
        } else {
            // Non-subscription licenses are considered expired immediately, no grace period is required
            self.is_expired()
        }
    }

    // Checks if License tier is lower than specified minimum
    //
    // Ordering is implemented by the `LicenseTier` enum itself
    #[must_use]
    pub(crate) fn is_lower_tier(&self, minimum_tier: LicenseTier) -> bool {
        self.tier < minimum_tier
    }
}

/// Exchange the currently stored key for a new one from the license server.
///
/// Doesn't update the cached license, nor does it save the new key in the database.
async fn renew_license() -> Result<String, LicenseError> {
    debug!("Exchanging license for a new one...");
    let Some(old_license_key) = Settings::get_current_settings().license else {
        return Err(LicenseError::LicenseNotFound);
    };

    let client = reqwest::Client::new();

    let request_body = RefreshRequestResponse {
        key: old_license_key,
    };

    let new_license_key =
        match client
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
                    error!("Failed to parse the response from the license server while trying to \
                        renew the license: {err}");
                    LicenseError::LicenseServerError(err.to_string())
                })?;
                    response.key
                }
                status => {
                    let status_message = response.text().await.unwrap_or_default();
                    let message = format!(
                        "Failed to renew the license, the license server returned a status code \
                    {status} with error: {status_message}"
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
/// 2. Is the cached license past its maximum expiry date
/// 3. Does current object count exceed license limits
/// 4. Is the license of at least the specified tier (or higher)
pub(crate) fn validate_license(
    license: Option<&License>,
    counts: &Counts,
    minimum_tier: LicenseTier,
) -> Result<(), LicenseError> {
    debug!("Validating if the license is present, not expired and not exceeding limits...");
    match license {
        Some(license) => {
            if license.is_max_overdue() {
                return Err(LicenseError::LicenseExpired);
            }
            if counts.is_over_license_limits(license) {
                return Err(LicenseError::LicenseLimitsExceeded);
            }
            if license.is_lower_tier(minimum_tier) {
                return Err(LicenseError::LicenseTierTooLow);
            }
            Ok(())
        }
        None => Err(LicenseError::LicenseNotFound),
    }
}

/// Helper function to save the license key string in the database
async fn save_license_key(pool: &PgPool, key: &str) -> Result<(), LicenseError> {
    debug!("Saving the license key to the database...");
    let mut settings = Settings::get_current_settings();
    settings.license = Some(key.to_string());
    update_current_settings(pool, settings).await?;

    info!("Successfully saved license key to the database.");

    Ok(())
}

/// Helper function to update the in-memory cached license mutex.
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
const MAX_OVERDUE_TIME: TimeDelta = TimeDelta::days(14);

#[instrument(skip_all)]
pub async fn run_periodic_license_check(pool: &PgPool) -> Result<(), LicenseError> {
    let config = server_config();
    let mut check_period: Duration = *config.check_period;
    info!(
        "Starting periodic license renewal check every {}",
        format_duration(check_period)
    );
    loop {
        debug!("Checking the license status...");
        // Check if the license is present in the mutex, if not skip the check
        if get_cached_license().is_none() {
            debug!("No license found, skipping license check");
            sleep(*config.check_period_no_license).await;
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

            if let Some(license) = license.as_ref() {
                if license.requires_renewal() {
                    // check if we are pass the maximum expiration date, after which we don't
                    // want to try to renew the license anymore
                    if license.is_max_overdue() {
                        check_period = *config.check_period;
                        warn!(
                            "Your license has expired and reached its maximum overdue date, please contact sales at sales<at>defguard.net"
                        );
                        debug!("Changing check period to {}", format_duration(check_period));
                        false
                    } else {
                        debug!(
                            "License requires renewal, as it is about to expire and is not past the maximum overdue time"
                        );
                        true
                    }
                } else {
                    // This if is only for logging purposes, to provide more detailed information
                    if license.subscription {
                        debug!("License doesn't need to be renewed yet, skipping renewal check");
                    } else {
                        debug!("License is not a subscription, skipping renewal check");
                    }
                    false
                }
            } else {
                debug!("No license found, skipping license check");
                false
            }
        };

        if requires_renewal {
            info!("License requires renewal, renewing license...");
            check_period = *config.check_period_renewal_window;
            debug!("Changing check period to {}", format_duration(check_period));
            match renew_license().await {
                Ok(new_license_key) => match save_license_key(pool, &new_license_key).await {
                    Ok(()) => {
                        update_cached_license(Some(&new_license_key))?;
                        check_period = *config.check_period;
                        debug!("Changing check period to {}", format_duration(check_period));
                        info!("Successfully renewed the license");
                    }
                    Err(err) => {
                        error!(
                            "Couldn't save the newly fetched license key to the database, error: {}",
                            err
                        );
                    }
                },
                Err(err) => {
                    warn!(
                        "Failed to renew the license: {err}. Retrying in {}",
                        format_duration(check_period)
                    );
                }
            }
        }

        sleep(check_period).await;
    }
}

// Mock public key
#[cfg(test)]
pub(crate) const PUBLIC_KEY: &[u8] = include_bytes!("test_key.asc");

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_license() {
        let license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9BYAAoJEKiEbAECmoSEtuMEAJu+mQlHt+OsIb3DSiknwyB+Z3d/AtvaOxIrnGSgnpJ22jAwKTRfBrOJsJQr0dA9wB4yawbXGv6+m35QPABQdSM+clq7x5J2bxyhLla00O7cdf2BcdYmyBEv1D/ZIjT1XBFoYEXzwxniviNsw4ZJaRsRIylr7eWsTw1tu+8IF4/U";
        let license = License::from_base64(license).unwrap();
        assert_eq!(license.customer_id, "0c4dcb5400544d47ad8617fcdf2704cb");
        assert!(!license.subscription);
        assert_eq!(
            license.valid_until.unwrap(),
            Utc.with_ymd_and_hms(2024, 12, 26, 13, 57, 54).unwrap()
        );
        assert!(license.is_expired());

        let limits = license.limits.unwrap();
        assert_eq!(limits.users, 10);
        assert_eq!(limits.devices, 100);
        assert_eq!(limits.locations, 5);

        // pre-1.6 license defaults to Business tier
        assert_eq!(license.tier, LicenseTier::Business);
    }

    #[test]
    fn test_legacy_license() {
        // use license key generated before user/device/location limits were introduced
        let license = "CigKIDVhMGRhZDRiOWNmZTRiNzZiYjkzYmI1Y2Q5MGM2ZjdjGNaw1LsGErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCZ3fBjAAKCRCohGwBApqEhNX+A/9dQmucvCTm5ll9h7a8f1N7d7dAOQW8/xhVA4bZP3GATIya/RxZ+cp+oHRYvHwSiRG3smGbRzti9DdHaTC/X1nqjMvZ6M4pR+aBayFH7fSUQKRj5z40juZ/HTCH/236YG3IzUZmIasLYl8Em9AY3oobkkwh1Yw+v8XYaBTUsrOv9w==";
        let license = License::from_base64(license).unwrap();
        assert_eq!(license.customer_id, "5a0dad4b9cfe4b76bb93bb5cd90c6f7c");
        assert!(!license.subscription);
        assert_eq!(
            license.valid_until.unwrap(),
            Utc.with_ymd_and_hms(2025, 1, 1, 10, 26, 30).unwrap()
        );

        assert!(license.is_expired());

        // legacy license is unlimited
        assert!(license.limits.is_none());

        // legacy license defaults to Business tier
        assert_eq!(license.tier, LicenseTier::Business);
    }

    #[test]
    fn test_new_license() {
        // This key has an additional test_field in the metadata that doesn't exist in the proto definition
        // It should still be able to decode the license correctly
        let license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9EMAAoJEKiEbAECmoSE/0kEAIb18pVTEYWQo0w6813nShJqi7++Uo/fX4pxaAzEiG9r5HGpZSbsceCarMiK1rBr93HOIMeDRsbZmJBA/MAYGi32uXgzLE8fGSd4lcUPAbpvlj7KNvQNH6sMelzQVw+AJVY+IASqO84nfy92taEVagbLqIwl/eSQUnehJBS+B5/z";
        let license = License::from_base64(license).unwrap();

        assert_eq!(license.customer_id, "0c4dcb5400544d47ad8617fcdf2704cb");
        assert!(!license.subscription);
        assert_eq!(
            license.valid_until.unwrap(),
            Utc.with_ymd_and_hms(2024, 12, 26, 13, 57, 54).unwrap()
        );

        // pre-1.6 license defaults to Business tier
        assert_eq!(license.tier, LicenseTier::Business);
    }

    #[test]
    fn test_invalid_license() {
        let license = "CigKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCZ3ZjywAKCRCohGwBApqEhEwFBACpHDnIszU2+KZcGhi3kycd3a12PyXJuFhhY4cuSyC8YEND85BplSWK1L8nu5ghFULFlddXP9HTHdxhJbtx4SgOQ8pxUY3+OpBN4rfJOMF61tvMRLaWlz7FWm/RnHe8cpoAOYm4oKRS0+FA2qLThxSsVa+S907ty19c6mcDgi6V5g==";
        let license = License::from_base64(license).unwrap();
        let counts = Counts::default();
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_err());
        assert!(validate_license(None, &counts, LicenseTier::Business).is_err());

        // One day past the expiry date, non-subscription license
        let license = License::new(
            "test".to_string(),
            false,
            Some(Utc::now() - TimeDelta::days(1)),
            None,
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_err());

        // One day before the expiry date, non-subscription license
        let license = License::new(
            "test".to_string(),
            false,
            Some(Utc::now() + TimeDelta::days(1)),
            None,
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_ok());

        // No expiry date, non-subscription license
        let license = License::new(
            "test".to_string(),
            false,
            None,
            None,
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_ok());

        // One day past the maximum overdue date
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() - MAX_OVERDUE_TIME - TimeDelta::days(1)),
            None,
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_err());

        // One day before the maximum overdue date
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
            None,
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_ok());

        let counts = Counts::new(5, 5, 5, 5);

        // Over object count limits
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
            Some(LicenseLimits {
                users: 1,
                devices: 1,
                locations: 1,
                network_devices: Some(1),
            }),
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_err());

        // Below object count limits
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
            Some(LicenseLimits {
                users: 10,
                devices: 10,
                locations: 10,
                network_devices: Some(10),
            }),
            None,
            LicenseTier::Business,
        );
        assert!(validate_license(Some(&license), &counts, LicenseTier::Business).is_ok());
    }

    #[test]
    fn test_license_tiers() {
        let legacy_license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9EMAAoJEKiEbAECmoSE/0kEAIb18pVTEYWQo0w6813nShJqi7++Uo/fX4pxaAzEiG9r5HGpZSbsceCarMiK1rBr93HOIMeDRsbZmJBA/MAYGi32uXgzLE8fGSd4lcUPAbpvlj7KNvQNH6sMelzQVw+AJVY+IASqO84nfy92taEVagbLqIwl/eSQUnehJBS+B5/z";
        let legacy_license = License::from_base64(legacy_license).unwrap();
        assert_eq!(legacy_license.tier, LicenseTier::Business);

        let business_license = "Ci4KJGEyYjE1M2MzLWYwZmEtNGUzNC05ZThkLWY0Nzk1NTA4OWMwNRiI7KTKBjABErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCaT/7iAAKCRCohGwBApqEhHdaA/0QqDNiryYSzWTEayBMwEBE6KAxTEtwRzXOxQxsnULjbQMol/SRjqfu8iwlI4IeBQP3CuAR9kglewvwg3osXDldIns46W/cDBd0jxANebLY9SPz0JS6pStMnSzhZ6rFW5ns3nCz86EOyAA9npx0/qxHCbtT6Qzi//5JYQe6VvvCmw==";
        let business_license = License::from_base64(business_license).unwrap();
        assert_eq!(business_license.tier, LicenseTier::Business);

        let enterprise_license = "Ci4KJDRiYjMzZTUyLWUzNGMtNGQyMS1iNDVhLTkxY2EzYTMzNGMwORiy7KTKBjACErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCaT/7sgAKCRCohGwBApqEhIMzBACGd7vIyLaRVGV/MAD8bpgWURG1x1tlxD9ehaSNkk01GkfZc+6+QwiTUBUOSp0MKPtuLmow5AIRKS9M75CQQ4bGtjLWO5cXJm1sduRpTvXwPLXNkRFPSxhjHmo4yjFFHMHMySqQE2WUjcz/b5dMT/WNqWYg7tSfT72eiK18eSVFTA==";
        let enterprise_license = License::from_base64(enterprise_license).unwrap();
        assert_eq!(enterprise_license.tier, LicenseTier::Enterprise);
    }
}
