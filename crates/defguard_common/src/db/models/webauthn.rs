use ctap_hid_fido2::public_key::{PublicKey, PublicKeyType};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool, Type, query, query_as, query_scalar};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatorInfo {
    pub name: Option<String>,
    pub kind: AuthenticatorKind,
    pub source: RegistrationSource,
}

impl AuthenticatorInfo {
    #[must_use]
    pub fn unattested(source: RegistrationSource) -> Self {
        Self {
            name: None,
            kind: AuthenticatorKind::Unknown,
            source,
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
}

impl WebAuthn {
    pub fn new(
        user_id: Id,
        name: String,
        passkey: &Passkey,
        authenticator: AuthenticatorInfo,
    ) -> Result<Self, ModelError> {
        let passkey = serde_cbor::to_vec(passkey).map_err(|_| ModelError::CannotCreate)?;
        Ok(Self {
            id: NoId,
            user_id,
            name,
            passkey,
            authenticator_name: authenticator.name,
            authenticator_kind: authenticator.kind,
            registration_source: authenticator.source,
        })
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
            registration_source \"registration_source: RegistrationSource\" \
            FROM webauthn WHERE user_id = $1",
            user_id
        )
        .fetch_all(pool)
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
