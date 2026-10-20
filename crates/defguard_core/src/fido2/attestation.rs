//! Hardware authenticators trusted to register FIDO2 keys from the desktop client.

#[cfg(any(test, feature = "test-support"))]
use std::sync::{PoisonError, RwLock};
use std::{borrow::Cow, collections::BTreeMap, sync::LazyLock};

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;
use thiserror::Error;
use webauthn_rs::prelude::{
    AttestationCaList, AttestationCaListBuilder, AttestationMetadata, AttestedPasskey,
    ParsedAttestationData, Uuid,
};
use x509_parser::{der_parser::der::parse_der_bitstring, parse_x509_certificate};

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
    #[error("attestation certificate is malformed")]
    MalformedCertificate,
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

static TRUSTED_AUTHENTICATORS: LazyLock<AttestationCaList> = LazyLock::new(|| {
    parse_catalog(TRUSTED_AUTHENTICATORS_JSON)
        .expect("bundled trusted authenticator list must parse")
});

#[cfg(all(feature = "test-support", not(debug_assertions)))]
compile_error!("test-support can inject trusted attestation CAs and must not reach release builds");

#[cfg(any(test, feature = "test-support"))]
static TEST_AUTHENTICATORS: LazyLock<RwLock<AttestationCaList>> =
    LazyLock::new(|| RwLock::new(AttestationCaListBuilder::new().build()));

#[cfg(not(any(test, feature = "test-support")))]
#[must_use]
pub fn trusted_attestation_cas() -> Cow<'static, AttestationCaList> {
    Cow::Borrowed(&TRUSTED_AUTHENTICATORS)
}

#[cfg(any(test, feature = "test-support"))]
#[must_use]
pub fn trusted_attestation_cas() -> Cow<'static, AttestationCaList> {
    let mut cas = TRUSTED_AUTHENTICATORS.clone();
    cas.union(
        &TEST_AUTHENTICATORS
            .read()
            .unwrap_or_else(PoisonError::into_inner),
    );
    Cow::Owned(cas)
}

#[must_use]
pub fn authenticator_description(aaguid: &Uuid) -> Option<String> {
    trusted_attestation_cas()
        .cas()
        .values()
        .find_map(|ca| ca.aaguids().get(aaguid))
        .map(|device| device.description_en().to_owned())
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
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
    TEST_AUTHENTICATORS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .union(&builder.build());
    Ok(())
}

/// Checks applied on top of webauthn-rs, which has already verified the attestation chain,
/// user verification and that the credential is not backup eligible.
pub fn verify_hardware_key(passkey: &AttestedPasskey) -> Result<HardwareKey, HardwareKeyError> {
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
            .map_err(|_| HardwareKeyError::MalformedCertificate)?;
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
        return true;
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
    use rcgen::{CertificateParams, CustomExtension, KeyPair};

    use super::*;

    #[test]
    fn test_bundled_list_parses() {
        let cas = parse_catalog(TRUSTED_AUTHENTICATORS_JSON).expect("bundled list parses");
        assert!(!cas.is_empty());
        assert!(cas.cas().values().all(|ca| !ca.blanket_allow()));
    }

    #[test]
    fn test_describes_bundled_models() {
        let yubikey_5_nfc = Uuid::parse_str("fa2b99dc-9e39-4257-8f92-4a30d23c4118").unwrap();
        assert_eq!(
            authenticator_description(&yubikey_5_nfc).as_deref(),
            Some("YubiKey 5 Series with NFC")
        );
        assert_eq!(authenticator_description(&Uuid::nil()), None);
    }

    fn certificate_der(transports: Option<&[u8]>) -> Vec<u8> {
        let mut params = CertificateParams::new(Vec::new()).expect("certificate params");
        if let Some(transports) = transports {
            params.custom_extensions = vec![CustomExtension::from_oid_content(
                &[1, 3, 6, 1, 4, 1, 45724, 2, 1, 1],
                transports.to_vec(),
            )];
        }
        let key_pair = KeyPair::generate().expect("key pair");
        params
            .self_signed(&key_pair)
            .expect("self signed certificate")
            .der()
            .to_vec()
    }

    #[test]
    fn test_roaming_check_reads_transports_extension() {
        const USB_ONLY: &[u8] = &[0x03, 0x02, 0x05, 0x20];
        const NFC_ONLY: &[u8] = &[0x03, 0x02, 0x04, 0x10];
        const INTERNAL_ONLY: &[u8] = &[0x03, 0x02, 0x03, 0x08];
        const NOT_A_BITSTRING: &[u8] = &[0x05, 0x00];

        assert!(!certificate_denies_roaming(&certificate_der(None)));
        assert!(!certificate_denies_roaming(&certificate_der(Some(
            USB_ONLY
        ))));
        assert!(!certificate_denies_roaming(&certificate_der(Some(
            NFC_ONLY
        ))));
        assert!(certificate_denies_roaming(&certificate_der(Some(
            INTERNAL_ONLY
        ))));
        assert!(certificate_denies_roaming(&certificate_der(Some(
            NOT_A_BITSTRING
        ))));
        assert!(certificate_denies_roaming(b"not a certificate"));
    }
}
