// use std::str::FromStr;

// use crate::{error::WebError, AppState};
// use axum::extract::{Query, State};
// use openidconnect::{
//     core::{CoreClient, CoreIdTokenVerifier, CoreProviderMetadata},
//     reqwest::async_http_client,
//     AdditionalClaims, AuthUrl, ClientId, GenderClaim, IdToken, IdTokenVerifier, IssuerUrl,
//     JsonWebKeySet, JsonWebKeyType, JweContentEncryptionAlgorithm, JwsSigningAlgorithm, Nonce,
//     NonceVerifier,
// };

// // /// Authorization Endpoint
// // /// See https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
// // pub async fn authorization(
// //     State(appstate): State<AppState>,
// //     Query(data): Query<AuthenticationRequest>,
// //     cookies: CookieJar,
// //     private_cookies: PrivateCookieJar,
// // ) -> Result<(StatusCode, HeaderMap, PrivateCookieJar), WebError> {

// //     Ok()
// // }

// /// Authentication response callback
// /// See https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
// #[derive(Deserialize, Serialize, Debug)]
// pub struct AuthenticationResponse {
//     // id_token: String,
//     code: String,
// }

// struct Verifier;

// impl NonceVerifier for Verifier {
//     fn verify(self, nonce: Option<&openidconnect::Nonce>) -> Result<(), String> {
//         Ok(())
//     }
// }

// pub async fn auth_callback(
//     State(state): State<AppState>,
//     Query(response): Query<AuthenticationResponse>,
// ) -> Result<(), WebError> {
//     // TODO(jck): log all useful stuff
//     debug!("OIDC login callback got response: {response:?}");
//     let code = response.code.clone();

//     info!("OIDC login callback got code: {code:?}");
//     // let token = IdToken::<
//     //     openidconnect::EmptyAdditionalClaims,
//     //     openidconnect::core::CoreGenderClaim,
//     //     openidconnect::core::CoreJweContentEncryptionAlgorithm,
//     //     openidconnect::core::CoreJwsSigningAlgorithm,
//     //     _,
//     // >::from_str(&response.code);
//     // debug!("Decoded token: {token:?}");
//     // if let Ok(token) = token {
//     //     // TOOD(jck): create user based on user info
//     //     // TOOD(jck): log user in
//     //     // token.claims(verifier, nonce_verifier);
//     //     // let provider_metadata = CoreProviderMetadata::discover(
//     //     //     &IssuerUrl::new("https://accounts.example.com".to_string())?,
//     //     //     http_client,
//     //     // )?;

//     //     // let client_id = ClientId::new("f2cef8b3-5b09-4c3f-988b-51fc3c42ecbc".to_string());
//     //     // let client_secret = None;
//     //     // let issuer_url = IssuerUrl::new(
//     //     //     "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/v2.0"
//     //     //         .to_string(),
//     //     // )
//     //     // .unwrap();
//     //     // let auth_url = AuthUrl::new(
//     //     //     "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/oauth2/v2.0/authorize".to_string()
//     //     // ).unwrap();
//     //     // let token_url = None;
//     //     // let userinfo_url = None;
//     //     // let jwks = JsonWebKeySet::default();
//     //     // let client = CoreClient::new(
//     //     //     client_id,
//     //     //     client_secret,
//     //     //     issuer_url,
//     //     //     auth_url,
//     //     //     token_url,
//     //     //     userinfo_url,
//     //     //     jwks,
//     //     // );
//     //     // let claims = token.claims(&client.id_token_verifier(), Verifier);
//     //     // info!("### Decoded claims: {claims:#?}");
//     //     // let name = claims.name().unwrap();
//     //     // let family_name = claims.family_name().unwrap();
//     //     // let given_name = claims.given_name().unwrap();
//     //     // let email = claims.email().unwrap();
//     //     // info!("### name: {name:?}, family_name: {family_name:?}, given_name: {given_name:?}, email: {email:?}");
//     //     let provider_metadata = CoreProviderMetadata::discover_async(
//     //         IssuerUrl::new(
//     //             // "https://login.microsoftonline.com/2fc43015-5699-4d01-bd01-d6f2bd66818a/v2.0"
//     //             "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
//     //         )
//     //         .unwrap(),
//     //         async_http_client,
//     //     )
//     //     .await
//     //     .unwrap();

//     //     let client = CoreClient::from_provider_metadata(provider_metadata, client_id, None);
//     //     let claims = token
//     //         .claims(&client.id_token_verifier(), &Nonce::new("ala".to_string()))
//     //         .unwrap();
//     //     info!("Decoded claims: {claims:#?}");
//     //     // // Set the URL the user will be redirected to after the authorization process.
//     //     // .set_redirect_uri(RedirectUrl::new("http://localhost:3000/api/v1/oidc/callback".to_string())?);
//     //     // TODO(jck): log all useful stuff
//     //     info!("OIDC login succeeded for user");
//     // } else {
//     //     // TODO(jck): add context
//     //     warn!("OIDC login failed");
//     // }

//     Ok(())
// }

use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use time::Duration;

use axum::extract::{Query, State};
use axum::Json;
use axum_client_ip::{InsecureClientIp, LeftmostXForwardedFor};
use axum_extra::extract::{CookieJar, PrivateCookieJar};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use openidconnect::core::{
    CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod, CoreGrantType,
    CoreIdTokenClaims, CoreIdTokenVerifier, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm, CoreResponseMode, CoreResponseType, CoreRevocableToken,
    CoreSubjectIdentifierType,
};
use openidconnect::{
    core::CoreProviderMetadata, reqwest::async_http_client, ClientId, ClientSecret, IssuerUrl,
    ProviderMetadata, RedirectUrl, RevocationUrl,
};
use openidconnect::{AuthenticationFlow, AuthorizationCode, CsrfToken, LanguageTag, Nonce, Scope};

use crate::appstate::AppState;
use crate::db::{AppEvent, DbPool, Session, SessionState, User, UserInfo};
use crate::enterprise::db::models::openid_provider::OpenIdProvider;
use crate::error::WebError;
use crate::handlers::{ApiResponse, AuthResponse, SESSION_COOKIE_NAME, SIGN_IN_COOKIE_NAME};
use crate::headers::{check_new_device_login, get_user_agent_device, parse_user_agent};
use crate::server_config;

type ProvMeta = ProviderMetadata<
    openidconnect::EmptyAdditionalProviderMetadata,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreClientAuthMethod,
    openidconnect::core::CoreClaimName,
    openidconnect::core::CoreClaimType,
    openidconnect::core::CoreGrantType,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJweKeyManagementAlgorithm,
    openidconnect::core::CoreJwsSigningAlgorithm,
    openidconnect::core::CoreJsonWebKeyType,
    openidconnect::core::CoreJsonWebKeyUse,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreResponseMode,
    openidconnect::core::CoreResponseType,
    openidconnect::core::CoreSubjectIdentifierType,
>;

async fn get_provider_metadata(url: &str) -> Result<ProvMeta, WebError> {
    let issuer_url = IssuerUrl::new(url.to_string()).unwrap();

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
        .await
        .unwrap();

    println!("{:?}", provider_metadata);

    Ok(provider_metadata)
}

async fn make_oidc_client(pool: &DbPool) -> Result<CoreClient, WebError> {
    let provider = OpenIdProvider::get_enabled(pool).await?;
    let provider_metadata = get_provider_metadata(&provider.provider_url).await?;

    let client_id = ClientId::new(provider.client_id);

    let client_secret = ClientSecret::new(provider.client_secret);

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(
                RedirectUrl::new("http://localhost:3000/api/v1/openid/callback".to_string())
                    .unwrap(),
            );

    Ok(client)
}

fn nonce_fn() -> Nonce {
    Nonce::new("nonce".to_string())
}

fn csrf_fn() -> CsrfToken {
    CsrfToken::new("csrf".to_string())
}

pub async fn make_auth_url(State(appstate): State<AppState>) -> Result<String, WebError> {
    // TODO(aleksander): make sure that the user enables the oidc login first
    let client = make_oidc_client(&appstate.pool).await?;

    let (authorize_url, csrf_state, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            csrf_fn,
            nonce_fn,
        )
        // This example is requesting access to the "calendar" features and the user's profile.
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    info!("{:?}", authorize_url);
    info!("{:?}", csrf_state);
    info!("{:?}", nonce);

    Ok(authorize_url.to_string())
}

/// Helper function to return redirection with status code 302.
fn redirect_to<T: AsRef<str>>(uri: T, cookies: CookieJar) -> (StatusCode, HeaderMap, CookieJar) {
    let mut headers = HeaderMap::new();
    headers.insert(
        LOCATION,
        HeaderValue::try_from(uri.as_ref()).expect("URI isn't a valid header value"),
    );

    (StatusCode::FOUND, headers, cookies)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthenticationResponse {
    code: AuthorizationCode,
    state: CsrfToken,
}

pub async fn auth_callback(
    cookies: CookieJar,
    user_agent: Option<TypedHeader<UserAgent>>,
    forwarded_for_ip: Option<LeftmostXForwardedFor>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    Query(params): Query<AuthenticationResponse>,
    State(appstate): State<AppState>,
) -> Result<(StatusCode, HeaderMap, CookieJar), WebError> {
    let client = make_oidc_client(&appstate.pool).await?;

    let token = client
        .exchange_code(params.code)
        .request_async(async_http_client)
        .await
        .unwrap();

    let nonce = Nonce::new("nonce".to_string());

    let token_verifier = client.id_token_verifier();
    let token_claims = token
        .extra_fields()
        .id_token()
        .expect("Server did not return an ID token")
        .claims(&token_verifier, &nonce)
        .unwrap();
    // println!("Google returned ID token: {:?}", token_claims);

    let email = token_claims.email().unwrap();
    let name = token_claims.name().unwrap();
    // TODO: check whats up with localized claims
    // println!("{:?}", token_claims);
    let given_name = token_claims
        .given_name()
        .unwrap()
        .clone()
        .into_iter()
        .next()
        .unwrap()
        .1;
    let family_name = token_claims
        .family_name()
        .unwrap()
        .clone()
        .into_iter()
        .next()
        .unwrap()
        .1;
    println!("Email: {:?}", email);
    println!("Name: {:?}", name);
    println!("Given Name: {:?}", given_name);
    println!("Family Name: {:?}", family_name);

    let username = email.split('@').next().unwrap();

    let user = match User::find_by_username(&appstate.pool, username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let mut user = User::new(
                username.to_string(),
                None,
                family_name.to_string(),
                given_name.to_string(),
                email.to_string(),
                // TODO: Add phone
                None,
            );
            user.openid_login = true;
            user.save(&appstate.pool).await?;
            user
        }
        Err(e) => {
            return Err(WebError::Authorization(e.to_string()));
        }
    };

    let ip_address = forwarded_for_ip.map_or(insecure_ip, |v| v.0).to_string();
    let user_agent_string = match user_agent {
        Some(value) => value.to_string(),
        None => String::new(),
    };
    let agent = parse_user_agent(&appstate.user_agent_parser, &user_agent_string);
    let device_info = agent.clone().map(|v| get_user_agent_device(&v));

    Session::delete_expired(&appstate.pool).await?;
    let session = Session::new(
        user.id.unwrap(),
        SessionState::PasswordVerified,
        ip_address.clone(),
        device_info,
    );
    session.save(&appstate.pool).await?;

    let max_age = Duration::seconds(server_config().auth_cookie_timeout.as_secs() as i64);
    let config = server_config();
    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.clone()))
        .domain(
            config
                .cookie_domain
                .clone()
                .expect("Cookie domain not found"),
        )
        .path("/")
        .http_only(true)
        .secure(!config.cookie_insecure)
        .same_site(SameSite::Lax)
        .max_age(max_age);
    let cookies = cookies.add(auth_cookie);

    let login_event_type = "AUTHENTICATION".to_string();

    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserCreated(user_info.clone()));

    check_new_device_login(
        &appstate.pool,
        &appstate.mail_tx,
        &session,
        &user,
        ip_address,
        login_event_type,
        agent,
    )
    .await?;

    Ok(redirect_to("/", cookies))
}
