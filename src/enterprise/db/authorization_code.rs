use crate::{db::DbPool, random::gen_alphanumeric};
use chrono::{Duration, Utc};
use model_derive::Model;
use oxide_auth::primitives::{
    generator::{RandomGenerator, TagGrant},
    grant::{Extensions, Grant},
};
use sqlx::{query_as, Error as SqlxError};

#[derive(Model)]
#[table(authorization_code)]
pub struct AuthorizationCode {
    id: Option<i64>,
    pub user_id: i64,
    pub client_id: String,
    pub code: String,
    pub redirect_uri: String,
    pub scope: String, // FIXME: is this needed? or use Vec<String>
    pub auth_time: i64,
    pub nonce: Option<String>,
}

impl AuthorizationCode {
    #[must_use]
    pub fn new(
        user_id: i64,
        client_id: String,
        redirect_uri: String,
        scope: String,
        nonce: Option<String>,
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
        }
    }

    /// Find by code.
    pub async fn find_code(pool: &DbPool, code: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, code, redirect_uri, scope, auth_time, nonce \
            FROM authorization_code WHERE code = $1",
            code
        )
        .fetch_optional(pool)
        .await
    }
}

impl From<Grant> for AuthorizationCode {
    fn from(grant: Grant) -> Self {
        let mut rnd = RandomGenerator::new(16);
        let code = rnd.tag(2, &grant).unwrap();
        Self {
            id: None,
            user_id: 0, // FIXME: grant.owner_id,
            client_id: grant.client_id,
            code,
            redirect_uri: grant.redirect_uri.to_string(),
            scope: grant.scope.to_string(),
            auth_time: Utc::now().timestamp(),
            nonce: None,
        }
    }
}

impl From<AuthorizationCode> for Grant {
    fn from(code: AuthorizationCode) -> Self {
        Self {
            owner_id: code.user_id.to_string(),
            client_id: code.client_id,
            scope: code.scope.parse().unwrap(),
            redirect_uri: code.redirect_uri.parse().unwrap(),
            until: Utc::now() + Duration::minutes(1),
            extensions: Extensions::new(),
        }
    }
}
