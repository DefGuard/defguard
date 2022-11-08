use crate::{auth::SESSION_TIMEOUT, db::DbPool};
use chrono::{Duration, NaiveDateTime, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{query, query_as, Error as SqlxError, Type};
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration};

#[derive(Clone, PartialEq, Type)]
#[repr(i16)]
pub enum SessionState {
    NotVerified,
    PasswordVerified,
    MultiFactorVerified,
}

#[derive(Clone)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub state: SessionState,
    pub created: NaiveDateTime,
    pub expires: NaiveDateTime,
    pub webauthn_challenge: Option<Vec<u8>>,
    pub web3_challenge: Option<String>,
}

impl Session {
    #[must_use]
    pub fn new(user_id: i64, state: SessionState) -> Self {
        let now = Utc::now();
        Self {
            id: thread_rng()
                .sample_iter(Alphanumeric)
                .take(24)
                .map(char::from)
                .collect(),
            user_id,
            state,
            created: now.naive_utc(),
            expires: (now + Duration::seconds(SESSION_TIMEOUT as i64)).naive_utc(),
            webauthn_challenge: None,
            web3_challenge: None,
        }
    }

    pub fn expired(&self) -> bool {
        self.expires < Utc::now().naive_utc()
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, user_id, state \"state: SessionState\", created, expires, webauthn_challenge, \
            web3_challenge FROM session WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn save(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO session (id, user_id, state, created, expires, webauthn_challenge, web3_challenge) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            self.id,
            self.user_id,
            self.state.clone() as i16,
            self.created,
            self.expires,
            self.webauthn_challenge,
            self.web3_challenge,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_state(&mut self, pool: &DbPool, state: SessionState) -> Result<(), SqlxError> {
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

    pub fn get_passkey_registration(&self) -> Option<PasskeyRegistration> {
        self.webauthn_challenge
            .as_ref()
            .and_then(|challenge| serde_cbor::from_slice(challenge).ok())
    }

    pub fn get_passkey_authentication(&self) -> Option<PasskeyAuthentication> {
        self.webauthn_challenge
            .as_ref()
            .and_then(|challenge| serde_cbor::from_slice(challenge).ok())
    }

    pub async fn set_passkey_authentication(
        &mut self,
        pool: &DbPool,
        passkey_auth: &PasskeyAuthentication,
    ) -> Result<(), SqlxError> {
        let webauthn_challenge = serde_cbor::to_vec(passkey_auth).unwrap();
        query!(
            "UPDATE session SET webauthn_challenge = $1 WHERE id = $2",
            webauthn_challenge,
            self.id
        )
        .execute(pool)
        .await?;
        self.webauthn_challenge = Some(webauthn_challenge);
        Ok(())
    }

    pub async fn set_passkey_registration(
        &mut self,
        pool: &DbPool,
        passkey_reg: &PasskeyRegistration,
    ) -> Result<(), SqlxError> {
        let webauthn_challenge = serde_cbor::to_vec(passkey_reg).unwrap();
        query!(
            "UPDATE session SET webauthn_challenge = $1 WHERE id = $2",
            webauthn_challenge,
            self.id
        )
        .execute(pool)
        .await?;
        self.webauthn_challenge = Some(webauthn_challenge);
        Ok(())
    }

    pub async fn set_web3_challenge(
        &mut self,
        pool: &DbPool,
        web3_challenge: String,
    ) -> Result<(), SqlxError> {
        query!(
            "UPDATE session SET web3_challenge = $1 WHERE id = $2",
            web3_challenge,
            self.id
        )
        .execute(pool)
        .await?;
        self.web3_challenge = Some(web3_challenge);
        Ok(())
    }

    pub async fn delete(self, pool: &DbPool) -> Result<(), SqlxError> {
        query!("DELETE FROM session WHERE id = $1", self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_expired(pool: &DbPool) -> Result<(), SqlxError> {
        query!("DELETE FROM session WHERE expires < now()",)
            .execute(pool)
            .await?;
        Ok(())
    }
}
