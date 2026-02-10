use axum::{Json, extract::State, http::StatusCode};
use axum_client_ip::InsecureClientIp;
use axum_extra::{
    TypedHeader,
    extract::{
        CookieJar, PrivateCookieJar,
        cookie::{Cookie, SameSite},
    },
    headers::UserAgent,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_common::{
    config::server_config,
    db::{
        Id,
        models::{Settings, settings::OpenIdUsernameHandling, user::User},
    },
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, RedirectUrl, Scope,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreUserInfoClaims},
};
use reqwest::Url;
use serde_json::json;
use sqlx::PgPool;
use time::Duration;

const COOKIE_MAX_AGE: Duration = Duration::days(1);
static CSRF_COOKIE_NAME: &str = "csrf";
static NONCE_COOKIE_NAME: &str = "nonce";
// The select_account prompt is not supported by all providers, most notably not by JumpCloud.
// Currently it's only enabled for Google, as it was tested to work there.
pub const SELECT_ACCOUNT_SUPPORTED_PROVIDERS: &[&str] = &["Google"];

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    enterprise::{
        db::models::openid_provider::OpenIdProvider,
        directory_sync::sync_user_groups_if_configured, ldap::utils::ldap_update_user_state,
        limits::update_counts,
    },
    error::WebError,
    handlers::{
        ApiResponse, AuthResponse, SESSION_COOKIE_NAME, SIGN_IN_COOKIE_NAME,
        auth::create_session,
        user::{MAX_USERNAME_CHARS, check_username},
    },
};

/// Prune the given username from illegal characters in accordance with the following rules:
///
/// To enable LDAP sync usernames need to avoid reserved characters.
/// Username requirements:
/// - 64 characters long
/// - only lowercase or uppercase latin alphabet letters (A-Z, a-z) and digits (0-9)
/// - starts with non-special character
/// - only special characters allowed: . - _
/// - no whitespaces
#[must_use]
pub fn prune_username(username: &str, handling: OpenIdUsernameHandling) -> String {
    let mut result = username.to_string();

    // Go through the string and remove any non-alphanumeric characters at the beginning
    result = result
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_string();

    let is_char_valid = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_';

    match handling {
        OpenIdUsernameHandling::RemoveForbidden => {
            result.retain(&is_char_valid);
        }
        OpenIdUsernameHandling::ReplaceForbidden => {
            result = result
                .chars()
                .map(|c| if is_char_valid(c) { c } else { '_' })
                .collect();
        }
        OpenIdUsernameHandling::PruneEmailDomain => {
            if let Some(at_index) = result.find('@') {
                result.truncate(at_index);
            }
            result = result
                .chars()
                .map(|c| if is_char_valid(c) { c } else { '_' })
                .collect();
        }
    }

    result.truncate(MAX_USERNAME_CHARS);
    result
}

/// Create HTTP client and prevent following redirects
fn get_async_http_client() -> Result<reqwest::Client, WebError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| {
            error!("Failed to create HTTP client: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })
}

async fn get_provider_metadata(url: &str) -> Result<CoreProviderMetadata, WebError> {
    let issuer_url = IssuerUrl::new(url.to_string()).map_err(|err| {
        WebError::BadRequest(format!(
            "Failed to create issuer URL from the provided URL: {url}. Error details: {err}",
        ))
    })?;
    let async_http_client = get_async_http_client()?;
    // Discover the provider metadata based on a known base issuer URL.
    // The URL should be in the form of e.g. https://accounts.google.com.
    // The URL shouldn't contain the ".well-known" part – it will be added automatically.
    match CoreProviderMetadata::discover_async(issuer_url, &async_http_client).await {
        Ok(provider_metadata) => Ok(provider_metadata),
        Err(err) => Err(WebError::Authorization(format!(
            "Failed to discover provider metadata, make sure the provider's URL is correct: {url}. \
            Error details: {err}",
        ))),
    }
}

/// Build a state with optional embedded data. Useful for passing additional information around the
/// authentication flow.
pub fn build_state(state_data: Option<String>) -> CsrfToken {
    let csrf_token = CsrfToken::new_random();
    if let Some(data) = state_data {
        let combined = format!("{}.{data}", csrf_token.secret());
        let encoded = BASE64_STANDARD.encode(combined);
        CsrfToken::new(encoded)
    } else {
        csrf_token
    }
}

/// Extract the state data from the provided state.
pub(crate) fn extract_state_data(state: &str) -> Option<String> {
    let decoded = BASE64_STANDARD.decode(state).ok()?;
    let decoded_str = String::from_utf8(decoded).ok()?;
    let result = decoded_str.split_once('.');
    if let Some((part1, part2)) = result {
        if part1.is_empty() {
            None
        } else {
            Some(part2.to_string())
        }
    } else {
        None
    }
}

/// Build OpenID Connect client.
/// `url`: redirect/callback URL
pub async fn make_oidc_client(
    url: Url,
    provider: &OpenIdProvider<Id>,
) -> Result<
    (
        ClientId,
        CoreClient<
            EndpointSet,
            EndpointNotSet,
            EndpointNotSet,
            EndpointNotSet,
            EndpointMaybeSet,
            EndpointMaybeSet,
        >,
    ),
    WebError,
> {
    let provider_metadata = get_provider_metadata(&provider.base_url).await?;
    let client_id = ClientId::new(provider.client_id.clone());
    let client_secret = ClientSecret::new(provider.client_secret.clone());
    let core_client = CoreClient::from_provider_metadata(
        provider_metadata,
        client_id.clone(),
        Some(client_secret),
    )
    .set_redirect_uri(RedirectUrl::from_url(url));

    Ok((client_id, core_client))
}

/// Get or create `User` from OpenID claims.
pub async fn user_from_claims(
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
    let async_http_client = get_async_http_client()?;
    // Exchange code for ID token.
    let token_response = match core_client
        .exchange_code(code)
        .map_err(|err| {
            error!("Failed to exchange code for ID token: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .request_async(&async_http_client)
        .await
    {
        Ok(token) => token,
        Err(err) => {
            return Err(WebError::Authorization(format!(
                "Failed to exchange code for ID token; OpenID provider error: {err}"
            )));
        }
    };
    let Some(id_token) = token_response.extra_fields().id_token() else {
        return Err(WebError::Authorization(
            "Server did not return an ID token".to_string(),
        ));
    };

    let access_token = token_response.access_token();

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
                .map(|aud| aud.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if audiences.len() > 1 {
        warn!(
            "OpenID claims: 'aud' should not contain these additional fields {}",
            audiences
                .iter()
                .filter(|&aud| **aud != *client_id)
                .map(|aud| aud.as_str())
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

    // Get the *sub* claim from the token.
    let sub = token_claims.subject().to_string();

    // Handle logging in or creating user.
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
                // User with the same email already exists, merge the accounts.
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
                let settings = Settings::get_current_settings();
                // Check if the user should be created, if doesn't exist (default: true).
                if !settings.openid_create_account {
                    warn!(
                        "User with email address {} is trying to log in through OpenID Connect \
                        for the first time, but the account creation is disabled. An enrollment \
                        should performed.",
                        email.as_str()
                    );
                    return Err(WebError::Authorization(
                        "User not found and the automatic account creation is disabled. \
                        Enable it or create the user."
                            .into(),
                    ));
                }

                // Try to get the username from `preferred_username` claim.
                // If it's not there, extract it from email.
                let username = if let Some(username) = token_claims.preferred_username() {
                    let username = username.as_str();
                    debug!(
                        "Preferred username {username} found in the claims. Using the username."
                    );
                    username
                } else {
                    debug!(
                        "Preferred username not found in the claims, extracting from email address."
                    );
                    // Extract the username from the email address
                    let username = email.split('@').next().ok_or(WebError::BadRequest(
                        "Failed to extract username from email address".to_string(),
                    ))?;
                    debug!("Username extracted from email ({email:?}): {username})");
                    username
                };

                let username = prune_username(username, settings.openid_username_handling);
                // Check if the username is valid just in case, not everything can be handled by the
                // pruning.
                check_username(&username)?;

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

                // Extract all necessary information from the token or call the userinfo endpoint.
                let given_name = token_claims
                    .given_name()
                    // `None` gets the default value from a localized claim.
                    // Otherwise, it is required to pass a locale.
                    .and_then(|claim| claim.get(None));
                let family_name = token_claims.family_name().and_then(|claim| claim.get(None));
                let phone = token_claims.phone_number();

                let userinfo_response: CoreUserInfoClaims;
                let (given_name, family_name, phone) = if let (
                    Some(given_name),
                    Some(family_name),
                    phone,
                ) = (given_name, family_name, phone)
                {
                    debug!("Given name and family name found in the claims for user {username}.");
                    (given_name, family_name, phone)
                } else {
                    debug!(
                        "Given name or family name not found in the claims for user {username}, \
                        trying to get them from the user info endpoint. Current values: \
                        given_name: {given_name:?}, family_name: {family_name:?}, phone: {phone:?}"
                    );

                    let async_http_client = get_async_http_client()?;

                    let retrieval_error = "Failed to retrieve given name and family name from \
                        provider's userinfo endpoint. Make sure you have configured your provider \
                        correctly and that you have granted the necessary permissions to retrieve \
                        such information from the token or the userinfo endpoint.";
                    userinfo_response = core_client
                        .user_info(access_token.clone(), Some(token_claims.subject().clone()))
                        .map_err(|err| {
                            error!(
                                "Failed to get family name and given name from provider's \
                                userinfo endpoint, they may not support this. Error details: {err}"
                            );
                            WebError::BadRequest(retrieval_error.into())
                        })?
                        .request_async(&async_http_client)
                        .await
                        .map_err(|err| {
                            error!(
                                "Failed to get family name and given name from provider's userinfo \
                                endpoint. Error details: {err}",
                            );
                            WebError::BadRequest(retrieval_error.into())
                        })?;

                    let claim_error = |claim_name: &str| {
                        format!(
                            "Failed to retrieve {claim_name} from provider's userinfo endpoint and \
                            the ID token.  Make sure you have configured your provider correctly \
                            and that you have granted the necessary permissions to retrieve such \
                            information from the token or the userinfo endpoint.",
                        )
                    };
                    let given_name = userinfo_response
                        .given_name()
                        .and_then(|claim| claim.get(None))
                        .ok_or(WebError::BadRequest(claim_error("given name")))?;
                    let family_name = userinfo_response
                        .family_name()
                        .and_then(|claim| claim.get(None))
                        .ok_or(WebError::BadRequest(claim_error("family name")))?;
                    let phone = userinfo_response.phone_number();

                    debug!(
                        "Given name and family name successfully retrieved from the user info \
                        endpoint for user {username}."
                    );

                    (given_name, family_name, phone)
                };

                let mut user = User::new(
                    username.clone(),
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
    let settings = Settings::get_current_settings();
    let (_client_id, client) = make_oidc_client(settings.callback_url()?, &provider).await?;

    // Generate the redirect URL and the values needed later for callback authenticity verification
    let mut authorize_url_builder = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()));

    if SELECT_ACCOUNT_SUPPORTED_PROVIDERS
        .iter()
        .any(|&p| provider.name.eq_ignore_ascii_case(p))
    {
        authorize_url_builder =
            authorize_url_builder.add_prompt(openidconnect::core::CoreAuthPrompt::SelectAccount);
    }

    let (authorize_url, csrf_state, nonce) = authorize_url_builder.url();

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
        ApiResponse::new(
            json!({"url": authorize_url, "button_display_name": provider.display_name}),
            StatusCode::OK,
        ),
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
    }

    private_cookies = private_cookies
        .remove(Cookie::from(NONCE_COOKIE_NAME))
        .remove(Cookie::from(CSRF_COOKIE_NAME));

    let config = server_config();
    let settings = Settings::get_current_settings();
    let mut user = user_from_claims(
        &appstate.pool,
        Nonce::new(cookie_nonce),
        payload.code,
        settings.callback_url()?,
    )
    .await?;

    let (session, user_info, mfa_info) =
        create_session(&appstate.pool, insecure_ip, user_agent.as_str(), &mut user).await?;

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

    // The user may not yet be authorized (pre-MFA), but syncing their groups should be fine here,
    // since he already managed to login through the provider. Currently, there is no other way to
    // sync the groups for the MFA enabled user logging in through the provider without firing it on
    // every login attempt, even for standard, non-provider users.
    if let Err(err) =
        sync_user_groups_if_configured(&user, &appstate.pool, &appstate.wireguard_tx).await
    {
        error!(
            "Failed to sync user groups for user {} with the directory while the user was trying \
            to login in through an external provider: {err}",
            user.username
        );
    } else {
        ldap_update_user_state(&mut user, &appstate.pool).await;
    }

    if let Some(mfa_info) = mfa_info {
        return Ok((
            cookies,
            private_cookies,
            ApiResponse::json(mfa_info, StatusCode::CREATED),
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
            ApiResponse::json(
                AuthResponse {
                    user: user_info,
                    url,
                },
                StatusCode::OK,
            ),
        ))
    } else {
        unimplemented!("Impossible to get here");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_prune_username() {
        // Test RemoveForbidden handling
        let handling_remove = OpenIdUsernameHandling::RemoveForbidden;
        assert_eq!(prune_username("zenek", handling_remove), "zenek");
        assert_eq!(prune_username("zenek34", handling_remove), "zenek34");
        assert_eq!(prune_username("zenek@34", handling_remove), "zenek34");
        assert_eq!(prune_username("first.last", handling_remove), "first.last");
        assert_eq!(prune_username("__zenek__", handling_remove), "zenek__");
        assert_eq!(prune_username("zenek?", handling_remove), "zenek");
        assert_eq!(prune_username("zenek!", handling_remove), "zenek");
        assert_eq!(
            prune_username(
                "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                handling_remove
            ),
            "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        );
        assert_eq!(prune_username("", handling_remove), "");
        assert_eq!(prune_username("!@#$%^&*()", handling_remove), "");
        assert_eq!(prune_username("!zenek", handling_remove), "zenek");
        assert_eq!(prune_username("...zenek", handling_remove), "zenek");

        // Test ReplaceForbidden handling
        let handling_replace = OpenIdUsernameHandling::ReplaceForbidden;
        assert_eq!(prune_username("zenek", handling_replace), "zenek");
        assert_eq!(prune_username("zenek34", handling_replace), "zenek34");
        assert_eq!(prune_username("zenek@34", handling_replace), "zenek_34");
        assert_eq!(prune_username("first.last", handling_replace), "first.last");
        assert_eq!(prune_username("__zenek__", handling_replace), "zenek__");
        assert_eq!(prune_username("zenek?", handling_replace), "zenek_");
        assert_eq!(prune_username("zenek!", handling_replace), "zenek_");
        assert_eq!(
            prune_username(
                "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                handling_replace
            ),
            "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        );

        // Test PruneEmailDomain handling
        let handling_prune_email = OpenIdUsernameHandling::PruneEmailDomain;
        assert_eq!(
            prune_username("zenek@example.com", handling_prune_email),
            "zenek"
        );
        assert_eq!(
            prune_username("user.name@domain.org", handling_prune_email),
            "user.name"
        );
        assert_eq!(
            prune_username("invalid!chars!@domain.com", handling_prune_email),
            "invalid_chars_"
        );
        assert_eq!(
            prune_username("multiple@at@domain.com", handling_prune_email),
            "multiple"
        );
        assert_eq!(
            prune_username(
                "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee@domain.com",
                handling_prune_email
            ),
            "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        );
    }

    #[test]
    fn test_state_build_and_extract() {
        // without data
        let token = build_state(None);
        let decoded = BASE64_STANDARD.decode(token.secret());
        // not base64 encoded
        assert!(decoded.is_err());
        assert!(!token.secret().is_empty());

        // with data
        let data = "somedata".to_string();
        let token = build_state(Some(data.clone()));
        let decoded = BASE64_STANDARD.decode(token.secret());
        assert!(decoded.is_ok());
        let decoded_str = String::from_utf8(decoded.unwrap()).unwrap();
        let (csrf, state_data) = decoded_str.split_once('.').unwrap();
        assert!(!csrf.is_empty());
        assert_eq!(state_data, data);

        // valid
        let data = "my_state_data".to_string();
        let token = build_state(Some(data.clone()));
        let extracted = extract_state_data(token.secret());
        assert_eq!(extracted, Some(data));

        // invalid base64
        let extracted = extract_state_data("not_base64!!");
        assert_eq!(extracted, None);

        // no dot
        let encoded = BASE64_STANDARD.encode("no_dot_here");
        let extracted = extract_state_data(&encoded);
        assert_eq!(extracted, None);

        // empty first part
        let encoded = BASE64_STANDARD.encode(".somedata");
        let extracted = extract_state_data(&encoded);
        assert_eq!(extracted, None);

        // empty second part
        let encoded = BASE64_STANDARD.encode("csrf.");
        let extracted = extract_state_data(&encoded);
        assert_eq!(extracted, Some(String::new()));

        // multiple dots
        let encoded = BASE64_STANDARD.encode("csrf.data.with.dots");
        let extracted = extract_state_data(&encoded);
        assert_eq!(extracted, Some("data.with.dots".to_string()));
    }
}
