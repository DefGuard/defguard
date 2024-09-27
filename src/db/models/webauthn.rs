use model_derive::Model;
use sqlx::{query, query_as, query_scalar, Error as SqlxError, PgExecutor, PgPool};
use webauthn_rs::prelude::Passkey;

use super::error::ModelError;
use crate::db::{Id, NoId};

#[derive(Model)]
pub struct WebAuthn<I = NoId> {
    id: I,
    pub(crate) user_id: Id,
    name: String,
    // serialize from/to [`Passkey`]
    pub passkey: Vec<u8>,
}

impl WebAuthn {
    pub fn new(user_id: Id, name: String, passkey: &Passkey) -> Result<Self, ModelError> {
        let passkey = serde_cbor::to_vec(passkey).map_err(|_| ModelError::CannotCreate)?;
        Ok(Self {
            id: NoId,
            user_id,
            name,
            passkey,
        })
    }
}

impl<I> WebAuthn<I> {
    /// Serialize [`Passkey`] from binary data.
    pub(crate) fn passkey(&self) -> Result<Passkey, ModelError> {
        let passkey =
            serde_cbor::from_slice(&self.passkey).map_err(|_| ModelError::CannotCreate)?;

        Ok(passkey)
    }
}

impl WebAuthn<Id> {
    /// Fetch all [`Passkey`]s for a given user.
    pub async fn passkeys_for_user(pool: &PgPool, user_id: Id) -> Result<Vec<Passkey>, SqlxError> {
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
    pub async fn all_for_user(pool: &PgPool, user_id: Id) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, user_id, name, passkey FROM webauthn WHERE user_id = $1",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    /// Delete all for a given user.
    pub async fn delete_all_for_user<'e, E>(executor: E, user_id: Id) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM webauthn WHERE user_id = $1", user_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
