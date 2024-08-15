use chrono::Utc;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

use super::DbPool;
use crate::random::gen_alphanumeric;

#[derive(Model, Clone)]
#[table(authorization_code)]
pub struct AuthCode {
    id: Option<i64>,
    pub user_id: i64,
    pub client_id: String,
    pub code: String,
    pub redirect_uri: String,
    pub scope: String,
    pub auth_time: i64,
    pub nonce: Option<String>,
    pub code_challenge: Option<String>,
}

impl AuthCode {
    #[must_use]
    pub fn new(
        user_id: i64,
        client_id: String,
        redirect_uri: String,
        scope: String,
        nonce: Option<String>,
        code_challenge: Option<String>,
    ) -> Self {
        let code = gen_alphanumeric(24);
        Self {
            id: None,
            user_id,
            client_id,
            code,
            redirect_uri,
            scope,
            auth_time: Utc::now().timestamp(),
            nonce,
            code_challenge,
        }
    }

    /// Find by code.
    pub async fn find_code(pool: &DbPool, code: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, code, redirect_uri, scope, auth_time, nonce, \
            code_challenge FROM authorization_code WHERE code = $1",
            code
        )
        .fetch_optional(pool)
        .await
    }

    // Remove a used authorization_code
    pub async fn consume(self, pool: &DbPool) -> Result<(), SqlxError> {
        self.delete(pool).await
    }
}
