use chrono::{NaiveDateTime, TimeDelta, Utc};
use sqlx::{query, query_as, Error as SqlxError, PgExecutor, PgPool, Type};
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration};

use crate::{db::Id, random::gen_alphanumeric, server_config};

#[derive(Clone, PartialEq, Type)]
#[repr(i16)]
pub enum SessionState {
    NotVerified,
    PasswordVerified,
    MultiFactorVerified,
    ApiTokenVerified,
}

// Representation of a Defguard server user session
// derived from session cookies
#[derive(Clone)]
pub struct Session {
    pub id: String,
    pub user_id: Id,
    pub state: SessionState,
    pub created: NaiveDateTime,
    pub expires: NaiveDateTime,
    pub webauthn_challenge: Option<Vec<u8>>,
    pub ip_address: String,
    pub device_info: Option<String>,
}

impl Session {
    #[must_use]
    pub fn new(
        user_id: Id,
        state: SessionState,
        ip_address: String,
        device_info: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let timeout = server_config().session_timeout;
        Self {
            id: gen_alphanumeric(24),
            user_id,
            state,
            created: now.naive_utc(),
            expires: (now + TimeDelta::seconds(timeout.as_secs() as i64)).naive_utc(),
            webauthn_challenge: None,
            ip_address,
            device_info,
        }
    }

    #[must_use]
    pub fn expired(&self) -> bool {
        self.expires < Utc::now().naive_utc()
    }

    pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, user_id, state \"state: SessionState\", created, expires, webauthn_challenge, \
            ip_address, device_info FROM session WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn save(&self, pool: &PgPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO session (id, user_id, state, created, expires, webauthn_challenge, ip_address, device_info) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            self.id,
            self.user_id,
            self.state.clone() as i16,
            self.created,
            self.expires,
            self.webauthn_challenge,
            self.ip_address,
            self.device_info,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_state(&mut self, pool: &PgPool, state: SessionState) -> Result<(), SqlxError> {
        query!(
            "UPDATE session SET state = $1 WHERE id = $2",
            state.clone() as i16,
            self.id
        )
        .execute(pool)
        .await?;
        self.state = state;

        Ok(())
    }

    #[must_use]
    pub fn get_passkey_registration(&self) -> Option<PasskeyRegistration> {
        self.webauthn_challenge
            .as_ref()
            .and_then(|challenge| serde_cbor::from_slice(challenge).ok())
    }

    #[must_use]
    pub fn get_passkey_authentication(&self) -> Option<PasskeyAuthentication> {
        self.webauthn_challenge
            .as_ref()
            .and_then(|challenge| serde_cbor::from_slice(challenge).ok())
    }

    pub async fn set_passkey_authentication<'e, E>(
        &mut self,
        executor: E,
        passkey_auth: &PasskeyAuthentication,
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if let Ok(webauthn_challenge) = serde_cbor::to_vec(passkey_auth) {
            query!(
                "UPDATE session SET webauthn_challenge = $1 WHERE id = $2",
                webauthn_challenge,
                self.id
            )
            .execute(executor)
            .await?;
            self.webauthn_challenge = Some(webauthn_challenge);
        }

        Ok(())
    }

    pub async fn set_passkey_registration<'e, E>(
        &mut self,
        executor: E,
        passkey_reg: &PasskeyRegistration,
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if let Ok(webauthn_challenge) = serde_cbor::to_vec(passkey_reg) {
            query!(
                "UPDATE session SET webauthn_challenge = $1 WHERE id = $2",
                webauthn_challenge,
                self.id
            )
            .execute(executor)
            .await?;
            self.webauthn_challenge = Some(webauthn_challenge);
        }

        Ok(())
    }

    pub async fn delete<'e, E>(self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM session WHERE id = $1", self.id)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn delete_expired<'e, E>(executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM session WHERE expires < now()",)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn delete_all_for_user<'e, E>(executor: E, user_id: i64) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM session WHERE user_id = $1", user_id)
            .execute(executor)
            .await?;

        Ok(())
    }
}
