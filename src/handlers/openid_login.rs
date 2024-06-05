use crate::{error::WebError, AppState};
use axum::extract::State;

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

pub async fn auth_callback(State(state): State<AppState>) -> Result<(), WebError> {
    // TODO(jck): log all useful stuff
    debug!("OIDC login callback");
    // TODO(jck): get user info from oidc provider
    // TOOD(jck): create user based on user info
    // TOOD(jck): log user in

    // TODO(jck): log all useful stuff
    info!("OIDC login succeeded for user");
    Ok(())
}
