use std::str::FromStr;

use crate::{error::WebError, AppState};
use axum::extract::{Query, State};
use openidconnect::{
    core::{CoreClient, CoreIdTokenVerifier, CoreProviderMetadata},
    reqwest::async_http_client,
    AdditionalClaims, AuthUrl, ClientId, GenderClaim, IdToken, IdTokenVerifier, IssuerUrl,
    JsonWebKeySet, JsonWebKeyType, JweContentEncryptionAlgorithm, JwsSigningAlgorithm, Nonce,
    NonceVerifier,
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

struct Verifier;

impl NonceVerifier for Verifier {
    fn verify(self, nonce: Option<&openidconnect::Nonce>) -> Result<(), String> {
        Ok(())
    }
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
        // let provider_metadata = CoreProviderMetadata::discover(
        //     &IssuerUrl::new("https://accounts.example.com".to_string())?,
        //     http_client,
        // )?;

        let client_id = ClientId::new("f2cef8b3-5b09-4c3f-988b-51fc3c42ecbc".to_string());
        // let client_secret = None;
        // let issuer_url = IssuerUrl::new(
        //     "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/v2.0"
        //         .to_string(),
        // )
        // .unwrap();
        // let auth_url = AuthUrl::new(
        //     "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/oauth2/v2.0/authorize".to_string()
        // ).unwrap();
        // let token_url = None;
        // let userinfo_url = None;
        // let jwks = JsonWebKeySet::default();
        // let client = CoreClient::new(
        //     client_id,
        //     client_secret,
        //     issuer_url,
        //     auth_url,
        //     token_url,
        //     userinfo_url,
        //     jwks,
        // );
        // let claims = token.claims(&client.id_token_verifier(), Verifier);
        // info!("### Decoded claims: {claims:#?}");
        // let name = claims.name().unwrap();
        // let family_name = claims.family_name().unwrap();
        // let given_name = claims.given_name().unwrap();
        // let email = claims.email().unwrap();
        // info!("### name: {name:?}, family_name: {family_name:?}, given_name: {given_name:?}, email: {email:?}");
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(
                "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/v2.0"
                    .to_string(),
            )
            .unwrap(),
            async_http_client,
        )
        .await
        .unwrap();

        let client = CoreClient::from_provider_metadata(provider_metadata, client_id, None);
        let claims = token.claims(&client.id_token_verifier(), &Nonce::new("ala".to_string())).unwrap();
        info!("Decoded claims: {claims:#?}");
        // // Set the URL the user will be redirected to after the authorization process.
        // .set_redirect_uri(RedirectUrl::new("http://localhost:3000/api/v1/oidc/callback".to_string())?);
        // TODO(jck): log all useful stuff
        info!("OIDC login succeeded for user");
    } else {
        // TODO(jck): add context
        warn!("OIDC login failed");
    }

    Ok(())
}
