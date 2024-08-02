use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rsa::pkcs8::DecodePublicKey;
use serde_cbor::Value;
// use rsa::{pkcs8::FromPublicKey, PaddingScheme, PublicKey, RsaPublicKey};

use crate::db::{DbPool, Settings};
use base64::prelude::*;
use pgp::{Deserializable, Message as PGPMessage, SignedPublicKey, StandaloneSignature};
use prost::Message;
use tonic::codec::{Decoder, Encoder};

// mod proto {
//     tonic::include_proto!("license");
// }
tonic::include_proto!("license");

#[allow(dead_code)]
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

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
+QmJLVFn5xVR6dGteYW7/05e
=06W7
-----END PGP PUBLIC KEY BLOCK-----
";

#[derive(Debug, Serialize, Deserialize)]
pub enum LicenseError {
    InvalidLicense,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshRequestResponse {
    key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct License {
    pub customer_id: String,
    pub subscription: bool,
    // TODO(aleksander): valid until should be optional?
    pub valid_until: DateTime<Utc>,
}

impl License {
    pub fn new(customer_id: String, subscription: bool, valid_until: DateTime<Utc>) -> Self {
        Self {
            customer_id,
            subscription,
            valid_until,
        }
    }

    /// Fetch a license from the license server
    async fn exchange(db_pool: &DbPool) -> Result<String, LicenseError> {
        let old_license_key = Settings::get_settings(db_pool)
            .await
            .unwrap()
            .license
            .unwrap();

        let client = reqwest::Client::new();

        let request_body = RefreshRequestResponse {
            key: old_license_key,
        };

        let new_license_key = match client
            .post("http://localhost:8002/api/license/refresh")
            .json(&request_body)
            .send()
            .await
        {
            Ok(response) => {
                let response: RefreshRequestResponse = response.json().await.unwrap();
                response.key
            }
            Err(_) => return Err(LicenseError::InvalidLicense),
        };

        Ok(new_license_key)
    }

    fn decode(bytes: &[u8]) -> Result<Vec<u8>> {
        let bytes = BASE64_STANDARD.decode(bytes).unwrap();
        Ok(bytes)
    }

    fn from_base64(key: &str) -> Result<License, LicenseError> {
        let bytes = key.as_bytes();
        let decoded = Self::decode(bytes).unwrap();
        let slice: &[u8] = &decoded;

        let license_key = LicenseKey::decode(slice).unwrap();
        let metadata = license_key.metadata.unwrap();
        let signature = license_key.signature.unwrap();

        let (public_key, _headers_public) = SignedPublicKey::from_string(PUBLIC_KEY).unwrap();

        let metadata_bytes = metadata.encode_to_vec();

        let sig = StandaloneSignature::from_bytes(signature.signature.as_slice()).unwrap();

        match sig.verify(&public_key, metadata_bytes.as_slice()) {
            Ok(_) => {
                println!("Signature is valid");

                Ok(License::new(
                    metadata.customer_id,
                    metadata.subscription,
                    DateTime::from_timestamp(metadata.valid_until, 0)
                        // check if we should really use unwrap or default
                        .unwrap_or_default(),
                ))
            }
            Err(_) => {
                println!("Signature is invalid");
                Err(LicenseError::InvalidLicense)
            }
        }
    }

    /// Load a license from the database
    async fn get_key_from_database(pool: &DbPool) -> Result<Option<String>, LicenseError> {
        let settings = Settings::get_settings(pool).await.unwrap();
        Ok(settings.license)
    }

    pub async fn load(pool: &DbPool) -> Result<Option<License>, LicenseError> {
        match Self::get_key_from_database(pool).await.unwrap() {
            Some(key) => Ok(Some(Self::from_base64(&key).unwrap())),
            None => {
                info!("No license key found in the database");
                Ok(None)
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        self.valid_until > Utc::now()
    }

    // pub async fn load(pool: &DbPool) -> Result<String, LicenseError> {
    //     let key = match Self::get_from_database(pool).await.unwrap() {
    //         Some(key) => key,
    //         None => return Err(LicenseError::InvalidLicense),
    //     };

    //     Self::from_base64(&key).unwrap();

    //     println!("{:?}", key);

    //     // let new_key = Self::exchange(pool).await.unwrap();

    //     // println!("{:?}", new_key);

    //     // let bytes = Self::fetch().await.unwrap();

    //     // let decoded = Self::decode(&bytes).unwrap();

    //     // let slice: &[u8] = &decoded;

    //     // let msg = PGPMessage::from_bytes(slice).unwrap();

    //     // let (public_key, _headers_public) = SignedPublicKey::from_string(PUBLIC_KEY).unwrap();

    //     // match msg.verify(&public_key) {
    //     //     Ok(_) => println!("Signature verified"),
    //     //     Err(e) => {
    //     //         println!("Signature not verified: {:?}", e);
    //     //         return Err(LicenseError::InvalidLicense);
    //     //     }
    //     // };

    //     // let content = msg.get_content().unwrap().unwrap();

    //     // let content = content.as_slice();

    //     // let license_metadata: LicenseMetadata = LicenseMetadata::decode(content).unwrap();

    //     // println!("{:?}", license_metadata);

    //     Ok("".to_string())
    // }

    // fn check(&self) -> Result<(), Error> {
    //     let (public_key, _headers_public) = SignedPublicKey::from_string(PUBLIC_KEY).unwrap();
    //     let (signature, _) = StandaloneSignature::from_string(SIGNATURE).unwrap();

    //     signature.verify(&public_key, &self.fields_concatenated()).unwrap();

    //     Ok(())
    // }
}
