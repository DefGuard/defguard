use chrono::{NaiveDateTime, Utc};
use ctap_hid_fido2::public_key::{PublicKey, PublicKeyType};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool, Type, query, query_as, query_scalar};
use uuid::Uuid;
use webauthn_rs::prelude::{COSEKeyType, ECDSACurve, EDDSACurve, Passkey};

use crate::db::{Id, NoId, models::ModelError};

/// Hardware only when a trusted vendor attestation was verified at registration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "webauthn_authenticator_kind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AuthenticatorKind {
    Hardware,
    Unknown,
}

/// Core is the Core web UI. The other sources are self-reported by the client app.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "webauthn_registration_source", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RegistrationSource {
    Core,
    Desktop,
    Mobile,
}

/// The trusted attestation that vouched for a key at registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationAnchor {
    pub aaguid: Uuid,
    pub valid_until: NaiveDateTime,
    pub chain: Vec<Vec<u8>>,
    pub chain_sha256: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorInfo {
    pub name: Option<String>,
    pub kind: AuthenticatorKind,
    pub source: RegistrationSource,
    pub anchor: Option<AttestationAnchor>,
}

impl AuthenticatorInfo {
    #[must_use]
    pub fn unattested(source: RegistrationSource) -> Self {
        Self {
            name: None,
            kind: AuthenticatorKind::Unknown,
            source,
            anchor: None,
        }
    }
}

#[derive(Model, Clone, Debug, PartialEq)]
pub struct WebAuthn<I = NoId> {
    pub id: I,
    pub user_id: Id,
    pub name: String,
    // serialize from/to [`Passkey`]
    pub passkey: Vec<u8>,
    pub authenticator_name: Option<String>,
    #[model(enum)]
    pub authenticator_kind: AuthenticatorKind,
    #[model(enum)]
    pub registration_source: RegistrationSource,
    pub attestation_aaguid: Option<Uuid>,
    pub attestation_valid_until: Option<NaiveDateTime>,
    #[model(option_ref)]
    pub attestation_chain: Option<Vec<Vec<u8>>>,
    #[model(option_ref)]
    pub attestation_chain_sha256: Option<Vec<String>>,
    pub registered_at: Option<NaiveDateTime>,
}

impl WebAuthn {
    pub fn new(
        user_id: Id,
        name: String,
        passkey: &Passkey,
        authenticator: AuthenticatorInfo,
    ) -> Result<Self, ModelError> {
        let passkey = serde_cbor::to_vec(passkey).map_err(|_| ModelError::CannotCreate)?;
        let mut key = Self {
            id: NoId,
            user_id,
            name,
            passkey,
            authenticator_name: authenticator.name,
            authenticator_kind: authenticator.kind,
            registration_source: authenticator.source,
            attestation_aaguid: None,
            attestation_valid_until: None,
            attestation_chain: None,
            attestation_chain_sha256: None,
            registered_at: Some(Utc::now().naive_utc()),
        };
        if let Some(anchor) = authenticator.anchor {
            key.attestation_aaguid = Some(anchor.aaguid);
            key.attestation_valid_until = Some(anchor.valid_until);
            key.attestation_chain = Some(anchor.chain);
            key.attestation_chain_sha256 = Some(anchor.chain_sha256);
        }
        Ok(key)
    }
}

impl<I> WebAuthn<I> {
    /// Serialize [`Passkey`] from binary data.
    pub fn passkey(&self) -> Result<Passkey, ModelError> {
        let passkey =
            serde_cbor::from_slice(&self.passkey).map_err(|_| ModelError::CannotCreate)?;

        Ok(passkey)
    }
}

impl WebAuthn<Id> {
    /// Fetch the user's passkeys, all of them when kind is None.
    pub async fn passkeys_for_user(
        pool: &PgPool,
        user_id: Id,
        kind: Option<AuthenticatorKind>,
    ) -> sqlx::Result<Vec<Passkey>> {
        let bin_keys = query_scalar!(
            "SELECT passkey FROM webauthn WHERE user_id = $1 \
            AND ($2::webauthn_authenticator_kind IS NULL OR authenticator_kind = $2)",
            user_id,
            kind as Option<AuthenticatorKind>
        )
        .fetch_all(pool)
        .await?;
        deserialize_passkeys(&bin_keys)
    }

    /// Check whether a user has at least one security key, of any kind when kind is None.
    pub async fn exists_for_user<'e, E>(
        executor: E,
        user_id: Id,
        kind: Option<AuthenticatorKind>,
    ) -> sqlx::Result<bool>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM webauthn WHERE user_id = $1 \
            AND ($2::webauthn_authenticator_kind IS NULL OR authenticator_kind = $2))",
            user_id,
            kind as Option<AuthenticatorKind>
        )
        .fetch_one(executor)
        .await
        .map(Option::unwrap_or_default)
    }

    /// Fetch all for a given user.
    pub async fn all_for_user(pool: &PgPool, user_id: Id) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT id, user_id, name, passkey, authenticator_name, \
            authenticator_kind \"authenticator_kind: AuthenticatorKind\", \
            registration_source \"registration_source: RegistrationSource\", \
            attestation_aaguid, attestation_valid_until, \
            attestation_chain, attestation_chain_sha256, registered_at \
            FROM webauthn WHERE user_id = $1",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    /// Fetch keys whose attestation chain contains any of the given certificates.
    pub async fn find_by_attestation_certs<'e, E>(
        executor: E,
        cert_sha256: &[String],
    ) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, user_id, name, passkey, authenticator_name, \
            authenticator_kind \"authenticator_kind: AuthenticatorKind\", \
            registration_source \"registration_source: RegistrationSource\", \
            attestation_aaguid, attestation_valid_until, \
            attestation_chain, attestation_chain_sha256, registered_at \
            FROM webauthn WHERE attestation_chain_sha256 && $1",
            cert_sha256
        )
        .fetch_all(executor)
        .await
    }

    /// Delete all for a given user.
    pub async fn delete_all_for_user<'e, E>(executor: E, user_id: Id) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM webauthn WHERE user_id = $1", user_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}

fn deserialize_passkeys(bin_keys: &[Vec<u8>]) -> sqlx::Result<Vec<Passkey>> {
    bin_keys
        .iter()
        .map(|bin| serde_cbor::from_slice(bin).map_err(|err| sqlx::Error::Decode(Box::new(err))))
        .collect()
}

/// Convert a `PassKey` into a form that `ctap-hid-fido2`'s verifier expects.
#[must_use]
pub fn to_ctap_public_key(passkey: &Passkey) -> Option<PublicKey> {
    let cose = passkey.get_public_key();
    match &cose.key {
        COSEKeyType::EC_EC2(ec2) if ec2.curve == ECDSACurve::SECP256R1 => {
            let mut der = Vec::with_capacity(1 + ec2.x.len() + ec2.y.len());
            der.push(0x04); // uncompressed point
            der.extend_from_slice(&ec2.x);
            der.extend_from_slice(&ec2.y);
            Some(PublicKey::with_der(&der, PublicKeyType::Ecdsa256))
        }
        COSEKeyType::EC_OKP(okp) if okp.curve == EDDSACurve::ED25519 => {
            Some(PublicKey::with_der(&okp.x, PublicKeyType::Ed25519))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::{models::User, setup_pool};

    #[sqlx::test]
    async fn test_find_by_attestation_certs(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        let leaf_sha256 = "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4";
        let root_sha256 = "62760c6a6ef91679f454c8902b80fd009825b3f25da90f1fbace2ec6586cd5a8";
        let key = WebAuthn {
            id: NoId,
            user_id: user.id,
            name: "security-key".to_owned(),
            passkey: Vec::new(),
            authenticator_name: None,
            authenticator_kind: AuthenticatorKind::Hardware,
            registration_source: RegistrationSource::Desktop,
            attestation_aaguid: Some(Uuid::nil()),
            attestation_valid_until: Some(Utc::now().naive_utc()),
            attestation_chain: Some(vec![b"leaf".to_vec(), b"root".to_vec()]),
            attestation_chain_sha256: Some(vec![leaf_sha256.to_owned(), root_sha256.to_owned()]),
            registered_at: Some(Utc::now().naive_utc()),
        }
        .save(&pool)
        .await
        .unwrap();

        let found = WebAuthn::find_by_attestation_certs(
            &pool,
            &[root_sha256.to_owned(), "unknown".to_owned()],
        )
        .await
        .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, key.id);
        assert_eq!(found[0].attestation_chain, key.attestation_chain);
        assert_eq!(
            found[0].attestation_chain_sha256,
            key.attestation_chain_sha256
        );

        let found = WebAuthn::find_by_attestation_certs(&pool, &[leaf_sha256.to_owned()])
            .await
            .unwrap();
        assert_eq!(found.len(), 1);

        let found = WebAuthn::find_by_attestation_certs(&pool, &["unknown".to_owned()])
            .await
            .unwrap();
        assert!(found.is_empty());
    }
}
