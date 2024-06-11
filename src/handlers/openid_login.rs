use std::str::FromStr;

use crate::{error::WebError, AppState};
use axum::extract::{Query, State};
use openidconnect::{
    core::CoreIdTokenVerifier, AdditionalClaims, GenderClaim, IdToken, IdTokenVerifier, JsonWebKeyType, JweContentEncryptionAlgorithm, JwsSigningAlgorithm
};

// /// Authorization Endpoint
// /// See https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
// pub async fn authorization(
//     State(appstate): State<AppState>,
//     Query(data): Query<AuthenticationRequest>,
//     cookies: CookieJar,
//     private_cookies: PrivateCookieJar,
// ) -> Result<(StatusCode, HeaderMap, PrivateCookieJar), WebError> {

//     Ok()
// }

/// Authentication response callback
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
#[derive(Deserialize, Serialize, Debug)]
pub struct AuthenticationResponse {
    id_token: String,
}

pub async fn auth_callback(
    State(state): State<AppState>,
    Query(response): Query<AuthenticationResponse>,
) -> Result<(), WebError> {
    // TODO(jck): log all useful stuff
    debug!("OIDC login callback got response: {response:?}");
    let token = IdToken::<
        openidconnect::EmptyAdditionalClaims,
        openidconnect::core::CoreGenderClaim,
        openidconnect::core::CoreJweContentEncryptionAlgorithm,
        openidconnect::core::CoreJwsSigningAlgorithm,
        _,
    >::from_str(&response.id_token);
    debug!("Decoded token: {token:?}");
    if let Ok(token) = token {
        // TOOD(jck): create user based on user info
        // TOOD(jck): log user in
        // token.claims(verifier, nonce_verifier);

        // TODO(jck): log all useful stuff
        info!("OIDC login succeeded for user");
    } else {
        // TODO(jck): add context
        warn!("OIDC login failed");
    }

    Ok(())
}
