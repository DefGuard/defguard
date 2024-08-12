use axum::http::StatusCode;

use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::json;

use time::Duration;

use axum::extract::State;

use axum_client_ip::{InsecureClientIp, LeftmostXForwardedFor};
use axum_extra::extract::{CookieJar, PrivateCookieJar};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use openidconnect::core::{
    CoreClient, CoreGenderClaim, CoreJsonWebKeyType, CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm, CoreResponseType,
};
use openidconnect::{
    core::CoreProviderMetadata, reqwest::async_http_client, ClientId, ClientSecret, IssuerUrl,
    ProviderMetadata, RedirectUrl,
};
use openidconnect::{AuthenticationFlow, CsrfToken, EmptyAdditionalClaims, IdToken, Nonce, Scope};

use crate::appstate::AppState;
use crate::db::{DbPool, MFAInfo, Session, SessionState, Settings, User, UserInfo};
use crate::enterprise::db::models::openid_provider::OpenIdProvider;
use crate::error::WebError;
use crate::handlers::user::check_username;
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

    // Discover the provider metadata based on a known base issuer URL
    // The url should be in the form of e.g. https://accounts.google.com
    // The url shouldn't contain a .well-known part, it will be added automatically
    let provider_metadata = match CoreProviderMetadata::discover_async(
        issuer_url,
        async_http_client,
    )
    .await
    {
        Ok(metadata) => metadata,
        Err(_) => {
            return Err(WebError::Authorization(format!(
                "Failed to discover provider metadata, make sure the providers' url is correct: {url}",

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
                "OpenID provider not set".to_string(),
            ));
        }
    };

    let provider_metadata = get_provider_metadata(&provider.base_url).await?;
    let client_id = ClientId::new(provider.client_id);
    let client_secret = ClientSecret::new(provider.client_secret);
    let config = server_config();
    let url = format!("{}auth/callback", config.url);
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

    // Generate the redirect URL and the values needed later for callback authenticity verification
    let (authorize_url, csrf_state, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::Implicit(false),
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
        .same_site(SameSite::Strict)
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
        .same_site(SameSite::Strict)
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

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthenticationResponse {
    id_token: IdToken<
        EmptyAdditionalClaims,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
        CoreJsonWebKeyType,
    >,
    state: CsrfToken,
}

pub async fn auth_callback(
    cookies: CookieJar,
    private_cookies: PrivateCookieJar,
    user_agent: Option<TypedHeader<UserAgent>>,
    forwarded_for_ip: Option<LeftmostXForwardedFor>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(payload): Json<AuthenticationResponse>,
) -> Result<(CookieJar, PrivateCookieJar, ApiResponse), WebError> {
    debug!("Auth callback received, logging in user...");

    // Get the nonce and csrf cookies, we need them to verify the callback
    let mut private_cookies = private_cookies;
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

    // Verify the csrf token
    if *payload.state.secret() != cookie_csrf {
        return Err(WebError::Authorization("CSRF token mismatch".to_string()));
    };

    // Get the ID token and verify it against the nonce value received in the callback
    let client = make_oidc_client(&appstate.pool).await?;
    let nonce = Nonce::new(cookie_nonce);
    let token_verifier = client.id_token_verifier();
    let id_token = payload.id_token;

    private_cookies = private_cookies
        .remove(Cookie::from("nonce"))
        .remove(Cookie::from("csrf"));

    // claims = user attributes
    let token_claims = match id_token.claims(&token_verifier, &nonce) {
        Ok(claims) => claims,
        Err(error) => {
            return Err(WebError::Authorization(format!(
                "Failed to verify ID token, error: {error:?}",
            )));
        }
    };

    // Only email and username is required for user lookup and login
    let email = token_claims.email().ok_or(WebError::BadRequest(
        "Email not found in the information returned from provider.".to_string(),
    ))?;
    let username = email
        .split('@')
        .next()
        .ok_or(WebError::BadRequest(
            "Failed to extract username from email address".to_string(),
        ))?
        // + is not allowed in usernames, but fairly common in email addresses
        // TODO: Make this more robust, maybe trim everything that's forbidden in usernames
        .replace('+', "_");

    check_username(&username)?;

    // Handle logging in or creating the user
    let settings = Settings::get_settings(&appstate.pool).await?;
    let user = match User::find_by_email(&appstate.pool, email).await {
        Ok(Some(mut user)) => {
            // Make sure the user is not disabled
            if !user.is_active {
                return Err(WebError::Authorization("User is disabled".to_string()));
            }

            if !user.openid_login {
                user.openid_login = true;
                user.save(&appstate.pool).await?;
            }
            user
        }
        Ok(None) => {
            // Check if the user should be created if they don't exist (default: true)
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

                // Extract all necessary information from the token needed to create an account
                let given_name_error =
                    "Given name not found in the information returned from provider.";
                let given_name = token_claims
                    .given_name()
                    .ok_or(WebError::BadRequest(given_name_error.to_string()))?
                    // 'None' gets you the default value from a localized claim. Otherwise you would need to pass a locale.
                    .get(None)
                    .ok_or(WebError::BadRequest(given_name_error.to_string()))?;
                let family_name_error =
                    "Family name not found in the information returned from provider.";
                let family_name = token_claims
                    .family_name()
                    .ok_or(WebError::BadRequest(family_name_error.to_string()))?
                    .get(None)
                    .ok_or(WebError::BadRequest(family_name_error.to_string()))?;
                let phone = token_claims.phone_number();

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

    // Handle creating the session
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

    info!("Authenticated user {username} with external OpenID provider. Veryfing MFA status...");
    if user.mfa_enabled {
        if let Some(mfa_info) = MFAInfo::for_user(&appstate.pool, &user).await? {
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
            Ok((
                cookies,
                private_cookies,
                ApiResponse {
                    json: json!(mfa_info),
                    status: StatusCode::CREATED,
                },
            ))
        } else {
            error!("Couldn't fetch MFA info for user {username} with MFA enabled");
            Err(WebError::DbError("MFA info read error".into()))
        }
    } else {
        let user_info = UserInfo::from_user(&appstate.pool, &user).await?;

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

        if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
            debug!("Found openid session cookie.");
            let redirect_url = openid_cookie.value().to_string();
            Ok((
                cookies,
                private_cookies.remove(openid_cookie),
                ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: Some(redirect_url)
                    }),
                    status: StatusCode::OK,
                },
            ))
        } else {
            debug!("No OpenID session found");
            Ok((
                cookies,
                private_cookies,
                ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: None,
                    }),
                    status: StatusCode::OK,
                },
            ))
        }
    }
}
