use axum::{extract::State, http::StatusCode, Json};
use axum_client_ip::{InsecureClientIp, LeftmostXForwardedFor};
use axum_extra::{
    extract::{
        cookie::{Cookie, SameSite},
        CookieJar, PrivateCookieJar,
    },
    headers::UserAgent,
    TypedHeader,
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
};
use reqwest::Url;
use serde_json::json;
use sqlx::PgPool;
use time::Duration;

const COOKIE_MAX_AGE: Duration = Duration::days(1);
static CSRF_COOKIE_NAME: &str = "csrf";
static NONCE_COOKIE_NAME: &str = "nonce";

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    db::{Id, Settings, User},
    enterprise::{db::models::openid_provider::OpenIdProvider, limits::update_counts},
    error::WebError,
    handlers::{
        auth::create_session,
        user::{check_username, prune_username},
        ApiResponse, AuthResponse, SESSION_COOKIE_NAME, SIGN_IN_COOKIE_NAME,
    },
    server_config,
};

async fn get_provider_metadata(url: &str) -> Result<CoreProviderMetadata, WebError> {
    let issuer_url = IssuerUrl::new(url.to_string()).unwrap();
    // Discover the provider metadata based on a known base issuer URL
    // The url should be in the form of e.g. https://accounts.google.com
    // The url shouldn't contain a .well-known part, it will be added automatically
    match CoreProviderMetadata::discover_async(issuer_url, async_http_client).await {
        Ok(provider_metadata) => Ok(provider_metadata),
        Err(err) => {
            Err(WebError::Authorization(format!(
                "Failed to discover provider metadata, make sure the provider's URL is correct: {url}. Error details: {err:?}",
            )))
        }
    }
}

/// Build OpenID Connect client.
/// `url`: redirect/callback URL
pub(crate) async fn make_oidc_client(
    url: Url,
    provider: &OpenIdProvider<Id>,
) -> Result<(ClientId, CoreClient), WebError> {
    let provider_metadata = get_provider_metadata(&provider.base_url).await?;
    let client_id = ClientId::new(provider.client_id.to_string());
    let client_secret = ClientSecret::new(provider.client_secret.to_string());
    let core_client = CoreClient::from_provider_metadata(
        provider_metadata,
        client_id.clone(),
        Some(client_secret),
    )
    .set_redirect_uri(RedirectUrl::from_url(url));

    Ok((client_id, core_client))
}

/// Get or create `User` from OpenID claims.
pub(crate) async fn user_from_claims(
    pool: &PgPool,
    nonce: Nonce,
    code: AuthorizationCode,
    callback_url: Url,
) -> Result<User<Id>, WebError> {
    let Some(provider) = OpenIdProvider::get_current(pool).await? else {
        return Err(WebError::ObjectNotFound(
            "OpenID provider not set".to_string(),
        ));
    };
    let (client_id, core_client) = make_oidc_client(callback_url, &provider).await?;
    // Exchange code for ID token.
    let token_response = match core_client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
    {
        Ok(token) => token,
        Err(err) => {
            return Err(WebError::Authorization(format!(
                "Failed to exchange code for ID token; OpenID provider error: {err:?}"
            )));
        }
    };
    let Some(id_token) = token_response.extra_fields().id_token() else {
        return Err(WebError::Authorization(
            "Server did not return an ID token".to_string(),
        ));
    };

    // Verify ID token against the nonce value received in the callback.
    let token_verifier = core_client
        .id_token_verifier()
        .require_audience_match(false);
    // claims = user attributes
    let token_claims = match id_token.claims(&token_verifier, &nonce) {
        Ok(claims) => claims,
        Err(error) => {
            return Err(WebError::Authorization(format!(
                "Failed to verify ID token, error: {error:?}",
            )));
        }
    };
    // Custom `aud` (audience) verfication. According to OpenID specification:
    // "The Client MUST validate that the aud (audience) Claim contains its client_id value
    // registered at the Issuer identified by the iss (issuer) Claim as an audience. The ID
    // Token MUST be rejected if the ID Token does not list the Client as a valid audience,
    // or if it contains additional audiences not trusted by the Client."
    // But some providers, like Zitadel, send additional values in `aud`, so allow that.
    let audiences = token_claims.audiences();
    if !audiences.iter().any(|aud| **aud == *client_id) {
        return Err(WebError::Authorization(format!(
            "Invalid OpenID claims: 'aud' must contain '{}' (found audiences: {})",
            client_id.as_str(),
            audiences
                .iter()
                .map(|aud| aud.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    };
    if audiences.len() > 1 {
        warn!(
            "OpenID claims: 'aud' should not contain these additional fields {}",
            audiences
                .iter()
                .filter(|&aud| **aud != *client_id)
                .map(|aud| aud.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Only email and username is required for user lookup and login
    let email = token_claims.email().ok_or(WebError::BadRequest(
        "Email not found in the information returned from provider. Make sure your provider is \
        configured correctly and that you have granted the necessary permissions to retrieve \
        such information."
            .to_string(),
    ))?;

    // Try to get the username from the preferred_username claim.
    // If it's not there, extract it from email.
    let username = if let Some(username) = token_claims.preferred_username() {
        debug!("Preferred username {username:?} found in the claims, extracting username from it.");
        username
    } else {
        debug!("Preferred username not found in the claims, extracting from email address.");
        // Extract the username from the email address
        let username = email.split('@').next().ok_or(WebError::BadRequest(
            "Failed to extract username from email address".to_string(),
        ))?;
        debug!("Username extracted from email ({email:?}): {username})");
        username
    };
    let username = prune_username(username);
    // Check if the username is valid just in case, not everything can be handled by the pruning.
    check_username(&username)?;

    // Get the *sub* claim from the token.
    let sub = token_claims.subject().to_string();

    // Handle logging in or creating user.
    let settings = Settings::get_settings(pool).await?;
    let user = match User::find_by_sub(pool, &sub)
        .await
        .map_err(|err| WebError::Authorization(err.to_string()))?
    {
        Some(user) => {
            debug!(
                "User {} is trying to log in using an OpenID provider.",
                user.username
            );
            // Make sure the user is not disabled
            if !user.is_active {
                debug!("User {} tried to log in, but is disabled", user.username);
                return Err(WebError::Authorization("User is disabled".into()));
            }
            user
        }
        None => {
            if let Some(mut user) = User::find_by_email(pool, email).await? {
                if !user.is_active {
                    debug!("User {} tried to log in, but is disabled", user.username);
                    return Err(WebError::Authorization("User is disabled".into()));
                }
                // User with the same email already exists, merge the accounts
                info!(
                    "User with email address {} is logging in through OpenID Connect for the \
                    first time and we've found an existing account with the same email \
                    address. Merging accounts.",
                    user.email
                );
                user.openid_sub = Some(sub);
                user.save(pool).await?;
                user
            } else {
                // Check if the user should be created if they don't exist (default: true)
                if !settings.openid_create_account {
                    warn!(
                        "User with email address {} is trying to log in through OpenID Connect \
                        for the first time, but the account creation is disabled. An enrollment \
                        should performed.",
                        email.as_str()
                    );
                    return Err(WebError::Authorization(
                        "User not found, but needs to be created in order to login using OIDC."
                            .into(),
                    ));
                }

                info!(
                    "User {username} is logging in through OpenID Connect for the first time and \
                    there is no account with the same email address ({}). Creating a new account.",
                    email.as_str()
                );
                // Check if user with the same username already exists (usernames are unique).
                if User::find_by_username(pool, &username).await?.is_some() {
                    return Err(WebError::Authorization(format!(
                        "User with username {username} already exists"
                    )));
                }

                // Extract all necessary information from the token needed to create an account
                let given_name_error =
                    "Given name not found in the information returned from provider. Make sure \
                    your provider is configured correctly and that you have granted the \
                    necessary permissions to retrieve such information.";
                let given_name = token_claims
                    .given_name()
                    // 'None' gets you the default value from a localized claim.
                    //  Otherwise you would need to pass a locale.
                    .and_then(|claim| claim.get(None))
                    .ok_or(WebError::BadRequest(given_name_error.into()))?;
                let family_name_error =
                    "Family name not found in the information returned from provider. Make sure \
                    your provider is configured correctly and that you have granted the \
                    necessary permissions to retrieve such information.";
                let family_name = token_claims
                    .family_name()
                    .and_then(|claim| claim.get(None))
                    .ok_or(WebError::BadRequest(family_name_error.into()))?;
                let phone = token_claims.phone_number();

                let mut user = User::new(
                    username.to_string(),
                    None,
                    family_name.to_string(),
                    given_name.to_string(),
                    email.to_string(),
                    phone.map(|v| v.to_string()),
                );
                user.openid_sub = Some(sub);
                user.save(pool).await?
            }
        }
    };

    update_counts(pool).await?;
    Ok(user)
}

pub(crate) async fn get_auth_info(
    _license: LicenseInfo,
    private_cookies: PrivateCookieJar,
    State(appstate): State<AppState>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    let provider = OpenIdProvider::get_current(&appstate.pool).await?;
    let Some(provider) = provider else {
        return Err(WebError::ObjectNotFound(
            "OpenID provider not set".to_string(),
        ));
    };

    let config = server_config();
    let (_client_id, client) = make_oidc_client(config.callback_url(), &provider).await?;

    // Generate the redirect URL and the values needed later for callback authenticity verification
    let (authorize_url, csrf_state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .url();

    let cookie_domain = config
        .cookie_domain
        .as_ref()
        .expect("Cookie domain not found");
    let nonce_cookie = Cookie::build((NONCE_COOKIE_NAME, nonce.secret().clone()))
        .domain(cookie_domain)
        .path("/api/v1/openid/callback")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(!config.cookie_insecure)
        .max_age(COOKIE_MAX_AGE)
        .build();
    let csrf_cookie = Cookie::build((CSRF_COOKIE_NAME, csrf_state.secret().clone()))
        .domain(cookie_domain)
        .path("/api/v1/openid/callback")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(!config.cookie_insecure)
        .max_age(COOKIE_MAX_AGE)
        .build();
    let private_cookies = private_cookies.add(nonce_cookie).add(csrf_cookie);

    Ok((
        private_cookies,
        ApiResponse {
            json: json!(
                {
                    "url": authorize_url,
                    "button_display_name": provider.display_name
                }
            ),
            status: StatusCode::OK,
        },
    ))
}

#[derive(Deserialize)]
pub(crate) struct AuthenticationResponse {
    code: AuthorizationCode,
    state: CsrfToken,
}

pub(crate) async fn auth_callback(
    _license: LicenseInfo,
    cookies: CookieJar,
    mut private_cookies: PrivateCookieJar,
    user_agent: TypedHeader<UserAgent>,
    forwarded_for_ip: Option<LeftmostXForwardedFor>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(payload): Json<AuthenticationResponse>,
) -> Result<(CookieJar, PrivateCookieJar, ApiResponse), WebError> {
    debug!("Auth callback received, logging in user...");

    // Get the nonce and CSRF cookies, we need them to verify the callback
    let cookie_nonce = private_cookies
        .get(NONCE_COOKIE_NAME)
        .ok_or(WebError::Authorization("Nonce cookie not found".into()))?
        .value_trimmed()
        .to_string();
    let cookie_csrf = private_cookies
        .get(CSRF_COOKIE_NAME)
        .ok_or(WebError::BadRequest("CSRF cookie not found".into()))?
        .value_trimmed()
        .to_string();

    // Verify the CSRF token
    if payload.state.secret() != &cookie_csrf {
        return Err(WebError::Authorization("CSRF token mismatch".into()));
    };

    private_cookies = private_cookies
        .remove(Cookie::from(NONCE_COOKIE_NAME))
        .remove(Cookie::from(CSRF_COOKIE_NAME));

    let config = server_config();
    let mut user = user_from_claims(
        &appstate.pool,
        Nonce::new(cookie_nonce),
        payload.code,
        config.callback_url(),
    )
    .await?;

    let ip_address = forwarded_for_ip.map_or(insecure_ip, |v| v.0);
    let (session, user_info, mfa_info) = create_session(
        &appstate.pool,
        &appstate.mail_tx,
        ip_address,
        user_agent.as_str(),
        &mut user,
    )
    .await?;

    let max_age = Duration::seconds(config.auth_cookie_timeout.as_secs() as i64);
    let cookie_domain = config
        .cookie_domain
        .as_ref()
        .expect("Cookie domain not found");
    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.id))
        .domain(cookie_domain)
        .path("/")
        .http_only(true)
        .secure(!config.cookie_insecure)
        .same_site(SameSite::Lax)
        .max_age(max_age);
    let cookies = cookies.add(auth_cookie);

    if let Some(mfa_info) = mfa_info {
        return Ok((
            cookies,
            private_cookies,
            ApiResponse {
                json: json!(mfa_info),
                status: StatusCode::CREATED,
            },
        ));
    }

    if let Some(user_info) = user_info {
        let url = if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
            debug!("Found OpenID session cookie, returning the redirect URL stored in it.");
            let url = openid_cookie.value().to_string();
            private_cookies = private_cookies.remove(openid_cookie);
            Some(url)
        } else {
            debug!("No OpenID session found, proceeding with login to Defguard.");
            None
        };

        Ok((
            cookies,
            private_cookies,
            ApiResponse {
                json: json!(AuthResponse {
                    user: user_info,
                    url
                }),
                status: StatusCode::OK,
            },
        ))
    } else {
        unimplemented!("Impossible to get here");
    }
}
