use chrono::Utc;
use model_derive::Model;
use sqlx::{PgExecutor, query_as};

use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};

#[derive(Model)]
#[table(authorization_code)]
pub(crate) struct AuthCode<I = NoId> {
    #[allow(dead_code)]
    id: I,
    pub(crate) user_id: Id,
    pub(crate) client_id: String,
    pub(crate) code: String,
    pub(crate) redirect_uri: String,
    pub(crate) scope: String,
    pub(crate) auth_time: i64,
    pub(crate) nonce: Option<String>,
    pub(crate) code_challenge: Option<String>,
}

impl AuthCode {
    #[must_use]
    pub(crate) fn new(
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

impl From<AuthCode<Id>> for AuthCode<NoId> {
    fn from(value: AuthCode<Id>) -> Self {
        Self {
            id: NoId,
            user_id: value.user_id,
            client_id: value.client_id,
            code: value.code,
            redirect_uri: value.redirect_uri,
            scope: value.scope,
            auth_time: value.auth_time,
            nonce: value.nonce,
            code_challenge: value.code_challenge,
        }
    }
}

impl AuthCode<Id> {
    /// Find by code.
    /// If found, delete `AuthCode` from the database right away, so it can't be reused.
    pub(crate) async fn find_code<'e, E>(
        executor: E,
        code: &str,
    ) -> Result<Option<AuthCode<NoId>>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "DELETE FROM authorization_code WHERE code = $1 \
            RETURNING id, user_id, client_id, code, redirect_uri, scope, auth_time, nonce, \
            code_challenge",
            code
        )
        .fetch_optional(executor)
        .await
        .map(|inner_option| inner_option.map(Into::into))
    }
}
