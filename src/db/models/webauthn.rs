use super::{error::ModelError, DbPool};
use model_derive::Model;
use sqlx::{query, query_as, query_scalar, Error as SqlxError};
use webauthn_rs::prelude::Passkey;

#[derive(Model)]
pub struct WebAuthn {
    id: Option<i64>,
    pub(crate) user_id: i64,
    name: String,
    // serialize from/to [`Passkey`]
    pub passkey: Vec<u8>,
}

impl WebAuthn {
    pub fn new(user_id: i64, name: String, passkey: &Passkey) -> Result<Self, ModelError> {
        let passkey = serde_cbor::to_vec(passkey).map_err(|_| ModelError::CannotCreate)?;
        Ok(Self {
            id: None,
            user_id,
            name,
            passkey,
        })
    }

    /// Serialize [`Passkey`] from binary data.
    pub(crate) fn passkey(&self) -> Result<Passkey, ModelError> {
        let passkey =
            serde_cbor::from_slice(&self.passkey).map_err(|_| ModelError::CannotCreate)?;
        Ok(passkey)
    }

    /// Fetch all [`Passkey`]s for a given user.
    pub async fn passkeys_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<Passkey>, SqlxError> {
        query_scalar!("SELECT passkey FROM webauthn WHERE user_id = $1", user_id)
            .fetch_all(pool)
            .await
            .map(|bin_keys| {
                bin_keys
                    .iter()
                    .map(|bin| serde_cbor::from_slice(bin).expect("Can't deserialize Passkey"))
                    .collect()
            })
    }

    /// Fetch all for a given user.
    pub async fn all_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, name, passkey FROM webauthn WHERE user_id = $1",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    /// Delete all for a given user.
    pub async fn delete_all_for_user(pool: &DbPool, user_id: i64) -> Result<(), SqlxError> {
        query!("DELETE FROM webauthn WHERE user_id = $1", user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
