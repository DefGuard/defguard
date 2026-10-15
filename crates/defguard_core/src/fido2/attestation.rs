//! Hardware authenticators trusted to register FIDO2 keys from the desktop client.

use std::{
    collections::BTreeMap,
    sync::{LazyLock, PoisonError, RwLock},
};

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;
use thiserror::Error;
use webauthn_rs::prelude::{
    AttestationCaList, AttestationCaListBuilder, AttestationMetadata, AttestedPasskey,
    ParsedAttestationData, RegisterPublicKeyCredential, Uuid,
};
use x509_parser::{der_parser::der::parse_der_bitstring, parse_x509_certificate};

use super::auth_flags::AuthFlags;

/// Filtered from the FIDO Metadata Service (MDS3) blob: certified, hardware protected roaming
/// authenticators with full basic or AttCA attestation.
const TRUSTED_AUTHENTICATORS_JSON: &str = include_str!("trusted_authenticators.json");

const FIDO_TRANSPORTS_OID: &str = "1.3.6.1.4.1.45724.2.1.1";
const BLUETOOTH_CLASSIC_BIT: usize = 0;
const NFC_BIT: usize = 3;

#[derive(Deserialize)]
struct Catalog {
    models: Vec<CatalogModel>,
}

#[derive(Deserialize)]
struct CatalogModel {
    aaguid: Uuid,
    description: String,
    roots: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TrustedListError {
    #[error("Invalid trusted authenticator list: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid root certificate encoding for {aaguid}: {source}")]
    Base64 {
        aaguid: Uuid,
        source: base64::DecodeError,
    },
    #[error("Invalid root certificate for {aaguid}: {message}")]
    Certificate { aaguid: Uuid, message: String },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HardwareKeyError {
    #[error("authenticator data is missing or malformed")]
    MalformedAuthData,
    #[error("user presence or user verification was not performed")]
    UserNotVerified,
    #[error("credential is backup eligible and may be synced off the device")]
    BackupEligible,
    #[error("attestation does not identify an authenticator model")]
    MissingAaguid,
    #[error("attestation certificate does not list a removable transport")]
    NotRoaming,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HardwareKey {
    pub aaguid: Uuid,
    pub description: Option<String>,
}

fn parse_catalog(json: &str) -> Result<AttestationCaList, TrustedListError> {
    let catalog: Catalog = serde_json::from_str(json)?;
    let mut builder = AttestationCaListBuilder::new();
    for model in catalog.models {
        for root in &model.roots {
            let der = BASE64_STANDARD
                .decode(root)
                .map_err(|source| TrustedListError::Base64 {
                    aaguid: model.aaguid,
                    source,
                })?;
            builder
                .insert_device_der(
                    &der,
                    model.aaguid,
                    model.description.clone(),
                    BTreeMap::new(),
                )
                .map_err(|err| TrustedListError::Certificate {
                    aaguid: model.aaguid,
                    message: err.to_string(),
                })?;
        }
    }
    Ok(builder.build())
}

static TRUSTED_AUTHENTICATORS: LazyLock<RwLock<AttestationCaList>> = LazyLock::new(|| {
    RwLock::new(
        parse_catalog(TRUSTED_AUTHENTICATORS_JSON)
            .expect("bundled trusted authenticator list must parse"),
    )
});

/// Every root is scoped to its own AAGUIDs, never blanket allowed.
#[must_use]
pub fn trusted_attestation_cas() -> AttestationCaList {
    TRUSTED_AUTHENTICATORS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

#[must_use]
pub fn authenticator_description(aaguid: &Uuid) -> Option<String> {
    TRUSTED_AUTHENTICATORS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .cas()
        .values()
        .find_map(|ca| ca.aaguids().get(aaguid))
        .map(|device| device.description_en().to_owned())
}

#[cfg(any(test, feature = "test-support"))]
pub fn trust_test_authenticator(
    ca_der: &[u8],
    aaguid: Uuid,
    description: &str,
) -> Result<(), TrustedListError> {
    let mut builder = AttestationCaListBuilder::new();
    builder
        .insert_device_der(ca_der, aaguid, description.to_owned(), BTreeMap::new())
        .map_err(|err| TrustedListError::Certificate {
            aaguid,
            message: err.to_string(),
        })?;
    TRUSTED_AUTHENTICATORS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .union(&builder.build());
    Ok(())
}

/// Checks applied on top of webauthn-rs, which has already verified the attestation chain.
pub fn verify_hardware_key(
    credential: &RegisterPublicKeyCredential,
    passkey: &AttestedPasskey,
) -> Result<HardwareKey, HardwareKeyError> {
    let flags =
        AuthFlags::from_registration(credential).ok_or(HardwareKeyError::MalformedAuthData)?;
    if !flags.present_and_verified() {
        return Err(HardwareKeyError::UserNotVerified);
    }
    if flags.backup_eligible {
        return Err(HardwareKeyError::BackupEligible);
    }

    let attestation = passkey.attestation();
    let aaguid = match &attestation.metadata {
        AttestationMetadata::Packed { aaguid } | AttestationMetadata::Tpm { aaguid, .. } => *aaguid,
        _ => return Err(HardwareKeyError::MissingAaguid),
    };

    let chain = match &attestation.data {
        ParsedAttestationData::Basic(chain)
        | ParsedAttestationData::AttCa(chain)
        | ParsedAttestationData::AnonCa(chain) => chain.as_slice(),
        _ => &[],
    };
    if let Some(leaf) = chain.first() {
        let der = leaf
            .to_der()
            .map_err(|_| HardwareKeyError::MalformedAuthData)?;
        if certificate_denies_roaming(&der) {
            return Err(HardwareKeyError::NotRoaming);
        }
    }

    Ok(HardwareKey {
        aaguid,
        description: authenticator_description(&aaguid),
    })
}

/// Many vendors omit the transports extension, so only an extension that lists no removable
/// transport counts against the key.
fn certificate_denies_roaming(der: &[u8]) -> bool {
    let Ok((_, certificate)) = parse_x509_certificate(der) else {
        return false;
    };
    let Some(extension) = certificate
        .extensions()
        .iter()
        .find(|extension| extension.oid.to_id_string() == FIDO_TRANSPORTS_OID)
    else {
        return false;
    };
    let Ok(transports) = parse_der_bitstring(extension.value)
        .map_err(|_| ())
        .and_then(|(_, object)| object.as_bitstring().map_err(|_| ()))
    else {
        return true;
    };
    !(BLUETOOTH_CLASSIC_BIT..=NFC_BIT).any(|bit| transports.is_set(bit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_list_parses() {
        let cas = parse_catalog(TRUSTED_AUTHENTICATORS_JSON).expect("bundled list parses");
        assert!(!cas.is_empty());
        assert!(cas.cas().values().all(|ca| !ca.blanket_allow()));
    }

    #[test]
    fn describes_bundled_models() {
        let yubikey_5_nfc = Uuid::parse_str("fa2b99dc-9e39-4257-8f92-4a30d23c4118").unwrap();
        assert_eq!(
            authenticator_description(&yubikey_5_nfc).as_deref(),
            Some("YubiKey 5 Series with NFC")
        );
        assert_eq!(authenticator_description(&Uuid::nil()), None);
    }
}
