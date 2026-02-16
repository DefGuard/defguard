use std::{fmt::Display, time::Duration};

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
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

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
=Xet7
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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum LicenseTier {
    Business, // this corresponds to both Team & Business level in our current pricing structure
    Enterprise,
}

impl Display for LicenseTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Business => {
                write!(f, "Business")
            }
            Self::Enterprise => {
                write!(f, "Enterprise")
            }
        }
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
        let (public_key, _headers_public) =
            SignedPublicKey::from_string(PUBLIC_KEY).expect("Failed to parse the public key");

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
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mI0EZ3ZfKQEEAKp7t6rldfVtMZ3x42cC+P7ZzF4OxuGlt/eDxoCzFpirCIwu1WY/
cpi+3zop0dovEBbIoYIHJVLwMxx/y/UzQ9H/3Vc0MZ3ZNwK+LRGugaOi6Y/Z6C3i
JjBJRMLi1rIU8TbYHE4QG6QUssDH74cE0s/WQjsEqkthkKwf5qv4/TgLABEBAAG0
HWRlZmd1YXJkLXRlc3QgPGRlZmd1YXJkQHRlc3Q+iNEEEwEIADsWIQSaLjwX4m6j
CO3NypmohGwBApqEhAUCZ3ZfKQIbAwULCQgHAgIiAgYVCgkICwIEFgIDAQIeBwIX
gAAKCRCohGwBApqEhONUA/9vnmAL8Roouk0GPeTKt9C/srXcmPtIadzoyEqGjsNI
Y1dpL7jhaKjY8sJtuNaCwTJ529w97fLM+SIeAMbwrK5naSdAIRqknn1h8a8VWkdX
isbqg9N/kMP891HyJZHM35VbHn1zFuJUh2gVzfIVaaAmC7YIMtmiAP5lYbrId/Ps
hw==
=6frq
-----END PGP PUBLIC KEY BLOCK-----
";

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
