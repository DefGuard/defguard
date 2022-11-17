use crate::{
    auth::SessionInfo,
    enterprise::{db::OAuth2Client, oauth_state::OAuthState},
    error::OriWebError,
    handlers::{user_for_admin_or_self, ApiResponse, ApiResult},
    license::PUBLIC_KEY,
    oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse},
};
use openidconnect::core::{CoreJsonWebKey, CoreJsonWebKeySet};
use oxide_auth_async::endpoint::{
    access_token::AccessTokenFlow, authorization::AuthorizationFlow, refresh::RefreshFlow,
};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    Data, State,
};
use rsa::{pkcs8::FromPublicKey, PublicKeyParts, RsaPublicKey};

#[get("/authorize")]
pub async fn authorize<'r>(
    oauth: OAuthRequest<'r>,
    state: &State<OAuthState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
    let mut flow = match AuthorizationFlow::prepare(state.inner().clone()) {
        Err(_) => unreachable!(),
        Ok(flow) => flow,
    };
    flow.execute(oauth).await
}

#[post("/authorize?<allow>")]
pub async fn authorize_consent<'r>(
    oauth: OAuthRequest<'r>,
    allow: Option<bool>,
    state: &State<OAuthState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
    let mut endpoint = state.inner().clone();
    endpoint.allow = allow.unwrap_or(false);
    endpoint.decision = true;
    let mut flow = match AuthorizationFlow::prepare(endpoint) {
        Err(_) => unreachable!(),
        Ok(flow) => flow,
    };
    flow.execute(oauth).await
}

#[post("/token", data = "<body>")]
pub async fn token<'r>(
    mut oauth: OAuthRequest<'r>,
    body: Data<'_>,
    state: &State<OAuthState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
    oauth.add_body(body).await;
    let mut flow = match AccessTokenFlow::prepare(state.inner().clone()) {
        Err(_) => unreachable!(),
        Ok(flow) => flow,
    };
    flow.execute(oauth).await
}

#[post("/refresh", data = "<body>")]
pub async fn refresh<'r>(
    mut oauth: OAuthRequest<'r>,
    body: Data<'_>,
    state: &State<OAuthState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
    oauth.add_body(body).await;
    let mut flow = match RefreshFlow::prepare(state.inner().clone()) {
        Err(_) => unreachable!(),
        Ok(flow) => flow,
    };
    flow.execute(oauth).await
}

#[post("/user/<username>/oauth2client", format = "json", data = "<data>")]
pub async fn add_oauth2client(
    session: SessionInfo,
    state: &State<OAuthState>,
    username: &str,
    data: Json<OAuth2Client>,
) -> ApiResult {
    let user = user_for_admin_or_self(&state.pool, &session, username).await?;
    let mut oauth2client = data.into_inner();
    if oauth2client.set_for_user(&state.pool, &user).await? {
        Ok(ApiResponse::default())
    } else {
        Err(OriWebError::Http(Status::NotFound))
    }
}

#[get("/discovery/keys")]
pub async fn discovery_keys() -> ApiResult {
    let public_key = RsaPublicKey::from_public_key_pem(PUBLIC_KEY).unwrap();
    let jwks = CoreJsonWebKeySet::new(vec![CoreJsonWebKey::new_rsa(
        public_key.n().to_bytes_be(),
        public_key.e().to_bytes_be(),
        None,
    )]);

    Ok(ApiResponse {
        json: json!(jwks),
        status: Status::Ok,
    })
}
