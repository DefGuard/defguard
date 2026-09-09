use std::{fmt, time::Duration};

use anyhow::Result;
use base64::prelude::*;
use chrono::{DateTime, TimeDelta, Utc};
use defguard_common::{
    VERSION,
    config::server_config,
    db::models::{
        Settings,
        gateway::Gateway,
        proxy::Proxy,
        settings::{SettingsSaveError, update_current_settings},
    },
    global_value,
    types::proxy::ProxyControlMessage,
};
use humantime::format_duration;
use pgp::{
    composed::{Deserializable, DetachedSignature, SignedPublicKey},
    types::KeyDetails,
};
use prost::Message;
use sqlx::PgPool;
use strum::VariantArray;
use thiserror::Error;
use tokio::time::sleep;

use crate::{
    enterprise::{
        db::models::enterprise_settings::unlicensed_edge_public_settings, limits::Counts,
    },
    grpc::proto::enterprise::license::{
        LicenseFeature as LicenseFeatureProto, LicenseKey, LicenseLimits, LicenseMetadata,
        LicenseTier as LicenseTierProto, SupportType as SupportTypeProto,
    },
    handlers::settings::public_settings_message,
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
// Mock public key
#[cfg(test)]
pub(crate) const PUBLIC_KEY: &[u8] = include_bytes!("test_key.asc");

#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("Provided signature does not match the license")]
    SignatureMismatch,
    #[error("Provided signature is invalid")]
    InvalidSignature,
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error(transparent)]
    SettingsSave(#[from] SettingsSaveError),
    #[error("License decoding error: {0}")]
    DecodeError(&'static str),
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
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LicenseTier {
    Business, // this corresponds to both Team & Business level in our current pricing structure
    Enterprise,
}

impl TryFrom<LicenseTierProto> for LicenseTier {
    type Error = LicenseError;

    fn try_from(value: LicenseTierProto) -> Result<Self, Self::Error> {
        match value {
            LicenseTierProto::Enterprise => Ok(Self::Enterprise),
            // fall back to Business tier for legacy licenses
            LicenseTierProto::Business | LicenseTierProto::Unspecified => Ok(Self::Business),
        }
    }
}

/// Represents support types
///
/// Variant order must be maintained to reflect protos order
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum SupportType {
    Free,
    Basic,
    Direct,
    BasicEnterprise,
    DirectEnterprise,
}

impl TryFrom<SupportTypeProto> for SupportType {
    type Error = LicenseError;

    fn try_from(value: SupportTypeProto) -> Result<Self, Self::Error> {
        match value {
            SupportTypeProto::Unspecified | SupportTypeProto::Free => Ok(Self::Free),
            SupportTypeProto::Basic => Ok(Self::Basic),
            SupportTypeProto::Direct => Ok(Self::Direct),
            SupportTypeProto::BasicEnterprise => Ok(Self::BasicEnterprise),
            SupportTypeProto::DirectEnterprise => Ok(Self::DirectEnterprise),
        }
    }
}

impl fmt::Display for LicenseTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Business => "Business",

            Self::Enterprise => "Enterprise",
        })
    }
}

impl LicenseTier {
    /// Returns the set of features that this tier grants by default, independently of any
    /// explicit additive flags on the license. Enterprise includes every feature; Business
    /// starts from an empty baseline so only explicitly granted flags are active.
    pub(crate) fn included_features(self) -> &'static [LicenseFeature] {
        match self {
            Self::Enterprise => LicenseFeature::VARIANTS,
            Self::Business => &[],
        }
    }
}

/// Additive, per-license feature grants. Each flag enables a single enterprise capability on
/// top of the license tier; it can only ever enable a feature, never restrict one.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, strum::VariantArray)]
pub enum LicenseFeature {
    ServiceLocations,
    DevicePosture,
    AclAllowedIps,
    ComponentHa,
}

impl TryFrom<LicenseFeatureProto> for LicenseFeature {
    type Error = LicenseError;

    fn try_from(value: LicenseFeatureProto) -> Result<Self, Self::Error> {
        match value {
            LicenseFeatureProto::ServiceLocations => Ok(Self::ServiceLocations),
            LicenseFeatureProto::DevicePosture => Ok(Self::DevicePosture),
            LicenseFeatureProto::AclAllowedIps => Ok(Self::AclAllowedIps),
            LicenseFeatureProto::ComponentHa => Ok(Self::ComponentHa),
            LicenseFeatureProto::Unspecified => {
                Err(LicenseError::DecodeError("Unspecified license feature"))
            }
        }
    }
}

/// Map raw protobuf feature values onto the domain enum, skipping unrecognized/unspecified ones
/// so an older Core can still decode a license that grants flags it doesn't know about yet.
fn map_license_features(values: &[i32]) -> Vec<LicenseFeature> {
    values
        .iter()
        .filter_map(|value| {
            let feature = LicenseFeatureProto::try_from(*value)
                .ok()
                .and_then(|proto| LicenseFeature::try_from(proto).ok());
            if feature.is_none() {
                debug!("Ignoring unknown license feature value {value}");
            }
            feature
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct License {
    pub customer_id: String,
    pub subscription: bool,
    pub valid_until: Option<DateTime<Utc>>,
    pub limits: Option<LicenseLimits>,
    pub version_date_limit: Option<DateTime<Utc>>,
    pub tier: LicenseTier,
    pub support_type: SupportType,
    pub features: Vec<LicenseFeature>,
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
        support_type: SupportType,
        features: Vec<LicenseFeature>,
    ) -> Self {
        Self {
            customer_id,
            subscription,
            valid_until,
            limits,
            version_date_limit,
            tier,
            support_type,
            features,
        }
    }

    #[must_use]
    pub(crate) fn has_feature(&self, feature: LicenseFeature) -> bool {
        self.features.contains(&feature)
    }

    fn decode(bytes: &[u8]) -> Result<Vec<u8>, LicenseError> {
        let bytes = BASE64_STANDARD.decode(bytes).map_err(|_| {
            LicenseError::DecodeError(
                "Failed to decode the license key, check if the provided key is correct.",
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
                        "Failed to find a signing key in the provided public key".to_owned(),
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
    pub(crate) fn from_base64(key: &str) -> Result<Self, LicenseError> {
        debug!("Decoding the license key from a provided base64 string...");
        let bytes = key.as_bytes();
        let decoded = Self::decode(bytes)?;
        let slice: &[u8] = &decoded;
        debug!("Decoded the license key, deserializing the license object...");

        let license_key = LicenseKey::decode(slice).map_err(|_| {
            LicenseError::DecodeError(
                "The license key is malformed, check if the provided key is correct.",
            )
        })?;
        let metadata_bytes: &[u8] = &license_key.metadata;
        let signature_bytes: &[u8] = &license_key.signature;
        debug!("Deserialized the license object, verifying the license signature...");

        match Self::verify_signature(metadata_bytes, signature_bytes) {
            Ok(()) => {
                info!("Successfully decoded the license and validated the license signature");
                let metadata = LicenseMetadata::decode(metadata_bytes).map_err(|_| {
                    LicenseError::DecodeError("Failed to decode the license metadata")
                })?;

                let valid_until = match metadata.valid_until {
                    Some(until) => DateTime::from_timestamp(until, 0),
                    None => None,
                };

                let version_date_limit = match metadata.version_date_limit {
                    Some(date) => DateTime::from_timestamp(date, 0),
                    None => None,
                };

                let license_tier = LicenseTierProto::try_from(metadata.tier)
                    .map_err(|err| {
                        error!("Failed to read license tier from license metadata: {err}");
                        LicenseError::DecodeError("Failed to decode license tier metadata")
                    })
                    .and_then(LicenseTier::try_from)?;

                let support_type = SupportTypeProto::try_from(metadata.support_type)
                    .map_err(|err| {
                        error!("Failed to read support type from license metadata: {err}");
                        LicenseError::DecodeError("Failed to decode support type metadata")
                    })
                    .and_then(SupportType::try_from)?;

                let features = map_license_features(&metadata.features);

                let license = Self::new(
                    metadata.customer_id,
                    metadata.subscription,
                    valid_until,
                    metadata.limits,
                    version_date_limit,
                    license_tier,
                    support_type,
                    features,
                );

                if license.requires_renewal() {
                    if license.is_max_overdue() {
                        warn!(
                            "The provided license has expired and reached its maximum overdue time, \
                            please contact sales<at>defguard.net"
                        );
                    } else {
                        warn!(
                            "The provided license is about to expire and requires a renewal. An \
                            automatic renewal process will attempt to renew the license soon. \
                            Alternatively, automatic renewal attempt will be also performed at the \
                            next Defguard start."
                        );
                    }
                }

                if !license.subscription && license.is_expired() {
                    warn!(
                        "The provided license is not a subscription and has expired, please \
                        contact sales<at>defguard.net"
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
    pub(crate) fn load() -> Result<Option<Self>, LicenseError> {
        if let Some(key) = Self::get_key() {
            Ok(Some(Self::from_base64(&key)?))
        } else {
            debug!("No license key found in the database");
            Ok(None)
        }
    }

    /// Try to load the license from the database, if the license requires a renewal, try to renew
    /// it. If the renewal fails, it will return the old license for the renewal service to renew it
    /// later.
    pub async fn load_or_renew(pool: &PgPool) -> Result<Option<Self>, LicenseError> {
        match Self::load()? {
            Some(license) => {
                if license.requires_renewal() {
                    if license.is_max_overdue() {
                        Err(LicenseError::LicenseExpired)
                    } else {
                        info!("License requires renewal, trying to renew it...");
                        match renew_license().await {
                            Ok(new_key) => {
                                let new_license = Self::from_base64(&new_key)?;
                                save_license_key(pool, &new_key).await?;
                                info!(
                                    "Successfully renewed and loaded the license, new license key \
                                    saved to the database"
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
    pub(crate) fn is_expired(&self) -> bool {
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
    pub(crate) fn time_overdue(&self) -> TimeDelta {
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
    pub(crate) fn requires_renewal(&self) -> bool {
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
    pub(crate) fn is_max_overdue(&self) -> bool {
        if self.subscription {
            self.time_overdue() > MAX_OVERDUE_TIME
        } else {
            // Non-subscription licenses are considered expired immediately, no grace period is
            // required.
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

    /// Helper function used to check if the cached license should be considered valid.
    ///
    /// This function checks the following two things:
    /// 1. Does the cached license exist
    /// 2. Is the cached license past its maximum expiry date
    /// 3. Does current object count exceed license limits
    /// 4. Is the license of at least the specified tier (or higher)
    pub(crate) fn validate(
        &self,
        counts: &Counts,
        minimum_tier: LicenseTier,
    ) -> Result<(), LicenseError> {
        debug!("Validating if the license is not expired and not exceeding limits");
        if self.is_max_overdue() {
            Err(LicenseError::LicenseExpired)
        } else if counts.is_over_license_limits(self) {
            Err(LicenseError::LicenseLimitsExceeded)
        } else if self.is_lower_tier(minimum_tier) {
            Err(LicenseError::LicenseTierTooLow)
        } else {
            Ok(())
        }
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

/// Helper function to save the license key string in the database
async fn save_license_key(pool: &PgPool, key: &str) -> Result<(), LicenseError> {
    debug!("Saving the license key to the database...");
    let mut settings = Settings::get_current_settings();
    settings.license = Some(key.to_owned());
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

/// Scale down enabled Gateways and Edges to one (per component).
async fn trim_gateways_and_edges(
    pool: &PgPool,
    proxy_control_tx: &tokio::sync::mpsc::Sender<ProxyControlMessage>,
) -> Result<(), LicenseError> {
    Gateway::leave_one_enabled(pool).await?;

    let edges = Proxy::leave_one_enabled(pool).await?;
    let count = edges.len();
    for mut edge in edges {
        edge.enabled = false;
        edge.save(pool).await?;
        if let Err(err) = proxy_control_tx
            .send(ProxyControlMessage::ShutdownConnection(edge.id))
            .await
        {
            error!(
                "Failed to shutdown Proxy {}, it may be disconnected: {err:?}",
                edge.id
            );
        }
    }

    debug!("Disabled {count} Edges");

    Ok(())
}

#[instrument(skip_all)]
pub async fn run_periodic_license_check(
    pool: &PgPool,
    proxy_control_tx: tokio::sync::mpsc::Sender<ProxyControlMessage>,
) -> Result<(), LicenseError> {
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

            trim_gateways_and_edges(pool, &proxy_control_tx).await?;

            sleep(*config.check_period_no_license).await;
            continue;
        }

        // Check if the license requires renewal, uses the cached value to be more efficient
        // The block here is to avoid holding the lock through awaits
        //
        // Multiple locks here may cause a race condition if the user decides to update the license
        // key while the renewal is in progress. However this seems like a rare case and shouldn't
        // be very problematic.
        let (requires_renewal, trim_components) = {
            let cached_license = get_cached_license();
            debug!("Checking if the license {cached_license:?} requires a renewal");

            if let Some(license) = cached_license.as_ref() {
                if license.requires_renewal() {
                    // check if we are pass the maximum expiration date, after which we don't
                    // want to try to renew the license anymore
                    if license.is_max_overdue() {
                        check_period = *config.check_period;
                        warn!(
                            "Your license has expired and reached its maximum overdue date, please \
                            contact sales at sales<at>defguard.net"
                        );
                        debug!("Changing check period to {}", format_duration(check_period));
                        (false, true)
                    } else {
                        debug!(
                            "License requires renewal, as it is about to expire and is not past \
                            the maximum overdue time"
                        );
                        (true, false)
                    }
                } else {
                    // This if is only for logging purposes, to provide more detailed information
                    if license.subscription {
                        debug!("License doesn't need to be renewed yet, skipping renewal check");
                    } else {
                        debug!("License is not a subscription, skipping renewal check");
                    }
                    (false, false)
                }
            } else {
                debug!("No license found, skipping license check");
                (false, true)
            }
        };

        if trim_components {
            trim_gateways_and_edges(pool, &proxy_control_tx).await?;

            // When the license is removed or expired, revert Edge UI controls
            // to their defaults so proxies no longer apply restricted settings.
            let settings = Settings::get_current_settings();
            if let Err(err) = proxy_control_tx
                .send(public_settings_message(unlicensed_edge_public_settings(
                    &settings,
                )))
                .await
            {
                error!("Failed to broadcast default public settings after license change: {err:?}");
            }
        }

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
                            "Couldn't save the newly fetched license key to the database, error: \
                            {err}"
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

#[cfg(test)]
mod tests;
