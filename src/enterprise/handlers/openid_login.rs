use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};

use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::json;

use time::Duration;

use axum::extract::{Query, State};

use axum_client_ip::{InsecureClientIp, LeftmostXForwardedFor};
use axum_extra::extract::{CookieJar, PrivateCookieJar};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use openidconnect::core::{CoreClient, CoreResponseType};
use openidconnect::{
    core::CoreProviderMetadata, reqwest::async_http_client, ClientId, ClientSecret, IssuerUrl,
    ProviderMetadata, RedirectUrl,
};
use openidconnect::{AuthenticationFlow, AuthorizationCode, CsrfToken, Nonce, Scope};

use crate::appstate::AppState;
use crate::db::{AppEvent, DbPool, Session, SessionState, Settings, User, UserInfo};
use crate::enterprise::db::models::openid_provider::OpenIdProvider;
use crate::error::WebError;
use crate::handlers::{ApiResponse, SESSION_COOKIE_NAME};
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

    let provider_metadata =
        match CoreProviderMetadata::discover_async(issuer_url, async_http_client).await {
            Ok(metadata) => metadata,
            Err(_) => {
                return Err(WebError::Authorization(format!(
                "Failed to discover provider metadata, make sure the providers' url is correct: {}",
                url
            )));
            }
        };

    Ok(provider_metadata)
}

async fn make_oidc_client(pool: &DbPool) -> Result<CoreClient, WebError> {
    let provider = match OpenIdProvider::get_current(pool).await? {
        Some(provider) => provider,
        None => {
            return Err(WebError::ObjectNotFound(
                "OpenID provider not found".to_string(),
            ));
        }
    };

    let provider_metadata = get_provider_metadata(&provider.base_url).await?;
    let client_id = ClientId::new(provider.client_id);
    let client_secret = ClientSecret::new(provider.client_secret);
    let config = server_config();
    let url = format!("{}api/v1/openid/callback", config.url);
    let redirect_url = match RedirectUrl::new(url) {
        Ok(url) => url,
        Err(err) => {
            error!("Failed to create redirect URL: {:?}", err);
            return Err(WebError::Authorization(
                "Failed to create redirect URL".to_string(),
            ));
        }
    };

    Ok(
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(redirect_url),
    )
}

pub async fn get_auth_info(
    private_cookies: PrivateCookieJar,
    State(appstate): State<AppState>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    let client = make_oidc_client(&appstate.pool).await?;

    let (authorize_url, csrf_state, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    let config = server_config();

    let nonce_cookie = Cookie::build(("nonce", nonce.secret().clone()))
        .domain(
            config
                .cookie_domain
                .clone()
                .expect("Cookie domain not found"),
        )
        .path("/api/v1/openid/callback")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .max_age(Duration::days(1))
        .build();

    let csrf_cookie = Cookie::build(("csrf", csrf_state.secret().clone()))
        .domain(
            config
                .cookie_domain
                .clone()
                .expect("Cookie domain not found"),
        )
        .path("/api/v1/openid/callback")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .max_age(Duration::days(1))
        .build();

    let private_cookies = private_cookies.add(nonce_cookie).add(csrf_cookie);

    Ok((
        private_cookies,
        ApiResponse {
            json: json!(
                {
                    "url": authorize_url,
                }
            ),
            status: StatusCode::OK,
        },
    ))
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
    private_cookies: PrivateCookieJar,
    user_agent: Option<TypedHeader<UserAgent>>,
    forwarded_for_ip: Option<LeftmostXForwardedFor>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    Query(params): Query<AuthenticationResponse>,
    State(appstate): State<AppState>,
) -> Result<(StatusCode, HeaderMap, CookieJar), WebError> {
    debug!("Auth callback received, logging in user...");
    let cookie_nonce = private_cookies
        .get("nonce")
        .ok_or(WebError::Authorization(
            "Nonce cookie not found".to_string(),
        ))?
        .value_trimmed()
        .to_string();

    let cookie_csrf = private_cookies
        .get("csrf")
        .ok_or(WebError::BadRequest("CSRF cookie not found".to_string()))?
        .value_trimmed()
        .to_string();

    if params.state.secret() != &cookie_csrf {
        return Err(WebError::Authorization("CSRF token mismatch".to_string()));
    };

    let client = make_oidc_client(&appstate.pool).await?;

    let token = client
        .exchange_code(params.code)
        .request_async(async_http_client)
        .await
        .map_err(|error| {
            WebError::Authorization(format!(
                "Failed to exchange code for token, error: {:?}",
                error
            ))
        })?;

    let nonce = Nonce::new(cookie_nonce);

    let token_verifier = client.id_token_verifier();
    let id_token = match token.extra_fields().id_token() {
        Some(token) => token,
        None => {
            return Err(WebError::Authorization(
                "Server did not return an ID token".to_string(),
            ));
        }
    };

    let token_claims = match id_token.claims(&token_verifier, &nonce) {
        Ok(claims) => claims,
        Err(error) => {
            return Err(WebError::Authorization(format!(
                "Failed to verify ID token, error: {:?}",
                error
            )));
        }
    };

    let email = token_claims.email().ok_or(WebError::BadRequest(
        "Email not found in the information returned from provider.".to_string(),
    ))?;

    // Extract given name and family name from the localized token claims
    // 'None' extracts the default value without the need to specify a language

    let given_name_error = "Given name not found in the information returned from provider.";

    let given_name = token_claims
        .given_name()
        .ok_or(WebError::BadRequest(given_name_error.to_string()))?
        // Gets the
        .get(None)
        .ok_or(WebError::BadRequest(given_name_error.to_string()))?;

    let family_name_error = "Family name not found in the information returned from provider.";

    let family_name = token_claims
        .family_name()
        .ok_or(WebError::BadRequest(family_name_error.to_string()))?
        .get(None)
        .ok_or(WebError::BadRequest(family_name_error.to_string()))?;

    let phone = token_claims.phone_number();

    let username = email
        .split('@')
        .next()
        .ok_or(WebError::BadRequest(
            "Failed to extract username from email address".to_string(),
        ))?
        // + is not allowed in usernames, but fairly common in email addresses
        // TODO: Make this more robust, trim everything that's forbidden in usernames
        .replace('+', "_");

    let settings = Settings::get_settings(&appstate.pool).await?;

    let user = match User::find_by_email(&appstate.pool, email).await {
        Ok(Some(mut user)) => {
            if !user.openid_login {
                user.openid_login = true;
                user.save(&appstate.pool).await?;
            }
            user
        }
        Ok(None) => {
            if settings.openid_create_account {
                // Check if user with the same username already exists
                if User::find_by_username(&appstate.pool, &username)
                    .await?
                    .is_some()
                {
                    return Err(WebError::Authorization(format!(
                        "User with username {} already exists",
                        username
                    )));
                }

                let mut user = User::new(
                    username.to_string(),
                    None,
                    family_name.to_string(),
                    given_name.to_string(),
                    email.to_string(),
                    phone.map(|v| v.to_string()),
                );
                user.openid_login = true;
                user.save(&appstate.pool).await?;
                user
            } else {
                return Err(WebError::Authorization(
                    "User not found. The user needs to be created first in order to login using OIDC."
                        .to_string(),
                ));
            }
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

    info!(
        "External OpenID authentication successful for user {}",
        user.username
    );

    Ok(redirect_to("/", cookies))
}
