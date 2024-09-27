use chrono::Utc;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgPool};

use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};

#[derive(Model, Clone)]
#[table(authorization_code)]
pub struct AuthCode<I = NoId> {
    id: I,
    pub user_id: Id,
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
        user_id: Id,
        client_id: String,
        redirect_uri: String,
        scope: String,
        nonce: Option<String>,
        code_challenge: Option<String>,
    ) -> Self {
        let code = gen_alphanumeric(24);
        Self {
            id: NoId,
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
}

impl AuthCode<Id> {
    /// Find by code.
    pub async fn find_code(pool: &PgPool, code: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id: _\", user_id, client_id, code, redirect_uri, scope, auth_time, nonce, \
            code_challenge FROM authorization_code WHERE code = $1",
            code
        )
        .fetch_optional(pool)
        .await
    }

    // Remove an used authorization_code
    pub async fn consume(self, pool: &PgPool) -> Result<(), SqlxError> {
        self.delete(pool).await
    }
}
