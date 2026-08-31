use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use axum::{
    Form,
    extract::{FromRef, FromRequestParts, Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, LOCATION},
        request::Parts,
    },
};
use axum_extra::extract::cookie::{Cookie, CookieJar, PrivateCookieJar, SameSite};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::Utc;
use defguard_common::db::{
    Id, NoId,
    models::{
        AuthCode, OAuth2AuthorizedApp, OAuth2Token, Session, SessionState, Settings, User,
        oauth2client::OAuth2Client,
    },
};
use openidconnect::{
    AccessToken, AdditionalClaims, Audience, AuthUrl, AuthorizationCode,
    EmptyAdditionalProviderMetadata, EmptyExtraTokenFields, EndUserEmail, EndUserFamilyName,
    EndUserGivenName, EndUserName, EndUserPhoneNumber, EndUserUsername, IdToken, IdTokenClaims,
    IdTokenFields, IssuerUrl, JsonWebKeySetUrl, LocalizedClaim, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, PrivateSigningKey, RefreshToken, ResponseTypes, Scope, StandardClaims,
    StandardErrorResponse, StandardTokenResponse, SubjectIdentifier, TokenUrl, UserInfoUrl,
    core::{
        CoreAuthErrorResponseType, CoreClaimName, CoreErrorResponseType, CoreGenderClaim,
        CoreGrantType, CoreHmacKey, CoreJsonWebKeySet, CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType, CoreRsaPrivateSigningKey,
        CoreSubjectIdentifierType, CoreTokenType,
    },
    url::Url,
};
use serde::{
    de::{Deserialize, Deserializer, Error as DeError, Unexpected, Visitor},
    ser::{Serialize, Serializer},
};
use sqlx::PgPool;
use thiserror::Error;

use super::{ApiErrorResponse, ApiResponse, ApiResult, SESSION_COOKIE_NAME};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, UserClaims},
    error::WebError,
    handlers::{SIGN_IN_COOKIE_MAX_AGE, SIGN_IN_COOKIE_NAME, cookie_domain},
    mail::templates::new_device_oidc_login_mail,
    server_config,
};

/// https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims
impl From<UserClaims> for StandardClaims<CoreGenderClaim> {
    fn from(user_claims: UserClaims) -> Self {
        let mut claims = Self::new(SubjectIdentifier::new(user_claims.sub));

        if let Some(name) = user_claims.name {
            let mut localized_claim = LocalizedClaim::new();
            localized_claim.insert(None, EndUserName::new(name));
            claims = claims.set_name(Some(localized_claim));
        }

        if let Some(given_name) = user_claims.given_name {
            let mut localized_claim = LocalizedClaim::new();
            localized_claim.insert(None, EndUserGivenName::new(given_name));
            claims = claims.set_given_name(Some(localized_claim));
        }

        if let Some(family_name) = user_claims.family_name {
            let mut localized_claim = LocalizedClaim::new();
            localized_claim.insert(None, EndUserFamilyName::new(family_name));
            claims = claims.set_family_name(Some(localized_claim));
        }

        if let Some(email) = user_claims.email {
            claims = claims.set_email(Some(EndUserEmail::new(email)));
        }

        if let Some(phone_number) = user_claims.phone_number {
            claims = claims.set_phone_number(Some(EndUserPhoneNumber::new(phone_number)));
        }

        if let Some(username) = user_claims.preferred_username {
            claims = claims.set_preferred_username(Some(EndUserUsername::new(username)));
        }

        claims
    }
}

/// Get the JSON Web Key Set used to verify ID token signatures
#[utoipa::path(
    get,
    path = "/api/v1/oauth/discovery/keys",
    tag = "OAuth2",
    responses(
        (status = 200, description = "JSON Web Key Set.", body = Object, example = json!({
            "keys": [{"kty": "RSA", "use": "sig", "alg": "RS256", "kid": "defguard", "n": "0vx7ago...", "e": "AQAB"}]
        })),
        (status = 500, description = "Unable to build key set.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn discovery_keys() -> ApiResult {
    let mut keys = Vec::new();
    if let Some(openid_key) = runtime_openid_key()? {
        keys.push(openid_key.as_verification_key());
    }

    Ok(ApiResponse::json(
        CoreJsonWebKeySet::new(keys),
        StatusCode::OK,
    ))
}
pub type DefguardIdTokenFields = IdTokenFields<
    GroupClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;

pub type DefguardTokenResponse = StandardTokenResponse<DefguardIdTokenFields, CoreTokenType>;
pub struct OAuth2ClientExtractor(Option<OAuth2Client<Id>>);

/// Errors arising from OpenID Connect flow helpers.
#[derive(Debug, Error)]
pub(crate) enum OidcFlowError {
    #[error("signing key unavailable: {0}")]
    SigningKey(String),
    #[error("invalid redirect URI")]
    InvalidRedirectUri,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("url error: {0}")]
    Url(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[allow(deprecated)]
fn runtime_openid_key() -> Result<Option<CoreRsaPrivateSigningKey>, OidcFlowError> {
    if server_config().hmac {
        Ok(None)
    } else {
        Settings::get_current_settings()
            .openid_key_required()
            .map(Some)
            .map_err(|err| {
                error!("OpenID signing key is unavailable: {err}");
                OidcFlowError::SigningKey(err.to_string())
            })
    }
}

/// Provide `OAuth2Client` when Basic Authorization header contains `client_id` and `client_secret`.
impl<S> FromRequestParts<S> for OAuth2ClientExtractor
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(basic_auth) = parts.headers.get(AUTHORIZATION).and_then(|value| {
            if let Ok(value) = value.to_str()
                && value.starts_with("Basic ")
            {
                return value.get(6..);
            }
            None
        }) {
            if let Ok(decoded) = BASE64_STANDARD.decode(basic_auth)
                && let Ok(auth_pair) = String::from_utf8(decoded)
                && let Some((client_id, client_secret)) = auth_pair.split_once(':')
            {
                let appstate = AppState::from_ref(state);
                let client = OAuth2Client::find_by_auth(&appstate.pool, client_id, client_secret)
                    .await
                    .map_err(WebError::from)?
                    .ok_or_else(|| WebError::Authorization("Invalid credentials".into()))?;
                return Ok(Self(Some(client)));
            }
            Err(WebError::Authorization("Invalid credentials".into()))
        } else {
            Ok(Self(None))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OAuthState(String);

impl Deref for OAuthState {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for OAuthState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

struct OAuthStateVisitor;

impl Visitor<'_> for OAuthStateVisitor {
    type Value = OAuthState;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a string containing only VSCHAR characters (%x20-7E)"
        )
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if !s.is_empty() && s.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            Ok(OAuthState(s.to_owned()))
        } else {
            Err(DeError::invalid_value(Unexpected::Str(s), &self))
        }
    }
}

/// Custom `Deserialize` implementation to enforce `state` parameter
/// validation per RFC 6749 Appendix A.5.
/// Only characters in the VSCHAR set (%x20-7E) are accepted.
impl<'de> Deserialize<'de> for OAuthState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(OAuthStateVisitor)
    }
}

/// List of values for "response_type" field.
struct FieldResponseTypes(Vec<CoreResponseType>);

impl Deref for FieldResponseTypes {
    type Target = Vec<CoreResponseType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FieldResponseTypes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for FieldResponseTypes {
    // serialize to a string with values separated by space
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let types: Vec<&str> = self.iter().map(CoreResponseType::as_ref).collect();
        serializer.serialize_str(&types.join(" "))
    }
}

struct FieldResponseTypesVisitor;

impl Visitor<'_> for FieldResponseTypesVisitor {
    type Value = FieldResponseTypes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a string containing `code`, `id_token`, or `token`"
        )
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let mut response_types = FieldResponseTypes(Vec::new());
        for value in s.split(' ') {
            match value {
                "code" => response_types.push(CoreResponseType::Code),
                "id_token" => response_types.push(CoreResponseType::IdToken),
                "token" => response_types.push(CoreResponseType::Token),
                _ => return Err(DeError::invalid_value(Unexpected::Str(s), &self)),
            }
        }
        Ok(response_types)
    }
}

impl<'de> Deserialize<'de> for FieldResponseTypes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FieldResponseTypesVisitor)
    }
}

/// Authentication Request
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
#[derive(Deserialize, Serialize)]
pub struct AuthenticationRequest {
    #[serde(default)]
    #[serde(skip_serializing)]
    allow: bool,
    scope: String,
    response_type: FieldResponseTypes,
    client_id: String,
    // client_secret: Option<String>,
    redirect_uri: String,
    state: Option<OAuthState>,
    // response_mode: Option<String>,
    nonce: Option<String>,
    // display: Option<String>,
    prompt: Option<String>,
    // max_age: Option<String>,
    // ui_locales: Option<String>,
    // id_token_hint: Option<String>,
    // login_hint: Option<String>,
    // acr_values: Option<String>,
    // PKCE
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

impl AuthenticationRequest {
    fn validate_for_client(
        &self,
        oauth2client: &OAuth2Client<Id>,
    ) -> Result<(), CoreAuthErrorResponseType> {
        // check scope: it is valid only if all requested scopes exist in the `oauth2client`
        if self
            .scope
            .split(' ')
            .any(|scope| !oauth2client.scope.iter().any(|s| s == scope))
        {
            error!(
                "Invalid scope for client {}: {}",
                oauth2client.name, self.scope
            );
            return Err(CoreAuthErrorResponseType::InvalidScope);
        }

        // currently we support only "code" for `response_type`
        if self.response_type.len() != 1 || !self.response_type.contains(&CoreResponseType::Code) {
            error!(
                "Invalid response_type for client {}, only 'code' supported",
                oauth2client.name
            );
            return Err(CoreAuthErrorResponseType::InvalidRequest);
        }

        // assume `client_id` is the same here and in `oauth2client`

        if !oauth2client.contains_redirect_url(&self.redirect_uri) {
            error!(
                "Invalid redirect_uri for client {}: {} not in [{}]",
                oauth2client.name,
                self.redirect_uri,
                oauth2client.redirect_uri.join(" "),
            );
            return Err(CoreAuthErrorResponseType::AccessDenied);
        }

        // check PKCE; currently, only SHA-256 method is supported
        // TODO: support `plain` which is the default if not specified
        if self.code_challenge.is_some() && self.code_challenge_method != Some("S256".to_owned()) {
            error!(
                "Invalid PKCE method: {:?}, only S256 supported",
                self.code_challenge_method
                    .as_ref()
                    .map_or("None", String::as_str),
            );
            return Err(CoreAuthErrorResponseType::InvalidRequest);
        }

        info!("Validation succeeded for client {}", oauth2client.name);

        Ok(())
    }
}

/// Helper function which creates redirect Uri with authorization code
async fn generate_auth_code_redirect(
    appstate: AppState,
    data: AuthenticationRequest,
    user_id: Id,
) -> Result<String, OidcFlowError> {
    let mut url = Url::parse(&data.redirect_uri).map_err(|_| OidcFlowError::InvalidRedirectUri)?;
    let auth_code = AuthCode::new(
        user_id,
        data.client_id,
        data.redirect_uri,
        data.scope,
        data.nonce,
        data.code_challenge,
    )
    .save(&appstate.pool)
    .await?;

    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("code", auth_code.code.as_str());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        }
    }

    Ok(url.to_string())
}

/// Helper function to return redirection with status code 302.
fn redirect_to<T: AsRef<str>>(
    uri: T,
    private_cookies: PrivateCookieJar,
) -> (StatusCode, HeaderMap, PrivateCookieJar) {
    let mut headers = HeaderMap::new();
    headers.insert(
        LOCATION,
        HeaderValue::try_from(uri.as_ref()).expect("URI isn't a valid header value"),
    );

    (StatusCode::FOUND, headers, private_cookies)
}

/// Helper function to redirect unauthorized user to login page
/// and store information about OpenID authorize url in cookie to redirect later
fn login_redirect(
    data: &AuthenticationRequest,
    private_cookies: PrivateCookieJar,
) -> Result<(StatusCode, HeaderMap, PrivateCookieJar), OidcFlowError> {
    let config = server_config();
    let settings = Settings::get_current_settings();
    let url = Settings::url().map_err(|e| OidcFlowError::Url(e.to_string()))?;
    let base_url = url.join("/api/v1/oauth/authorize").map_err(|err| {
        error!("Failed to prepare redirect URL: {err}");
        OidcFlowError::Internal(err.to_string())
    })?;
    let mut cookie = Cookie::build((
        SIGN_IN_COOKIE_NAME,
        format!(
            "{base_url}?{}",
            serde_urlencoded::to_string(data).unwrap_or_default()
        ),
    ))
    .path("/")
    .secure(
        config.cookie_insecure.map_or(
            settings
                .cookie_secure()
                .map_err(|e| OidcFlowError::Url(e.to_string()))?,
            |insecure| !insecure,
        ),
    )
    .same_site(SameSite::Lax)
    .http_only(true)
    .max_age(SIGN_IN_COOKIE_MAX_AGE);
    if let Some(cookie_domain) = cookie_domain() {
        cookie = cookie.domain(cookie_domain);
    }
    Ok(redirect_to("/auth/login", private_cookies.add(cookie)))
}

/// Start the OAuth2 authorization flow
///
/// Redirects to the login or consent page when the user is not authenticated or has not
/// yet approved the client. Implements the
/// [OpenID Connect authorization endpoint](https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint).
#[utoipa::path(
    get,
    path = "/api/v1/oauth/authorize",
    tag = "OAuth2",
    params(
        ("client_id" = String, Query, description = "ID of the OAuth2 client."),
        ("redirect_uri" = String, Query, description = "Redirect URI registered for the client."),
        ("response_type" = String, Query, description = "OAuth2 response type, for example `code`."),
        ("scope" = String, Query, description = "Space-separated list of requested scopes."),
        ("state" = String, Query, description = "Opaque value returned unchanged to the client."),
        ("nonce" = Option<String>, Query, description = "Value bound to the ID token to mitigate replay attacks."),
        ("code_challenge" = Option<String>, Query, description = "PKCE code challenge."),
        ("code_challenge_method" = Option<String>, Query, description = "PKCE code challenge method, for example `S256`."),
        ("prompt" = Option<String>, Query, description = "OpenID `prompt` parameter, for example `consent`."),
        ("allow" = Option<bool>, Query, description = "Set by the consent screen to allow or deny the request."),
    ),
    responses(
        (status = 302, description = "Redirect to the client, to the login page or to the consent page."),
        (status = 400, description = "Invalid authorization request.", body = ApiErrorResponse, example = json!({"msg": "Invalid redirect URI"})),
        (status = 500, description = "Unable to handle authorization request.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn authorization(
    State(appstate): State<AppState>,
    Query(data): Query<AuthenticationRequest>,
    cookies: CookieJar,
    private_cookies: PrivateCookieJar,
) -> Result<(StatusCode, HeaderMap, PrivateCookieJar), WebError> {
    let error;
    let mut is_redirect_allowed = false;
    if let Some(oauth2client) =
        OAuth2Client::find_by_client_id(&appstate.pool, &data.client_id).await?
    {
        is_redirect_allowed = oauth2client.contains_redirect_url(&data.redirect_uri);
        match (
            oauth2client.enabled,
            data.validate_for_client(&oauth2client),
        ) {
            (true, Ok(())) => {
                match &data.prompt {
                    Some(s) if s == "consent" => {
                        info!(
                            "Redirecting user to consent form - client id {}",
                            data.client_id
                        );
                        // FIXME: do not panic
                        return Ok(redirect_to(
                            format!("/consent?{}", serde_urlencoded::to_string(data).unwrap()),
                            private_cookies,
                        ));
                    }
                    Some(s) if s == "none" => {
                        error!("'none' prompt in client id {} request", data.client_id);
                        error = CoreAuthErrorResponseType::LoginRequired;
                    }
                    _ => {
                        return if let Some(session_cookie) = cookies.get(SESSION_COOKIE_NAME) {
                            if let Ok(Some(session)) =
                                Session::find_by_id(&appstate.pool, session_cookie.value()).await
                            {
                                // If session expired return login
                                if session.expired() {
                                    info!(
                                        "Session {} for user id {} has expired, redirecting to \
                                        login",
                                        session.id, session.user_id
                                    );
                                    let _result = session.delete(&appstate.pool).await;
                                    Ok(login_redirect(&data, private_cookies)?)
                                } else {
                                    let mut user =
                                        User::find_by_id(&appstate.pool, session.user_id)
                                            .await?
                                            .ok_or(WebError::Authorization(
                                                "User not found".into(),
                                            ))?;

                                    user.verify_mfa_state(&appstate.pool).await?;

                                    // Session exists even if user hasn't completed MFA verification
                                    // yet, thus we need to check if MFA is enabled and the
                                    // verification is done.
                                    if user.mfa_enabled
                                        && session.state != SessionState::MultiFactorVerified
                                    {
                                        info!(
                                            "MFA not verified for user id {}, redirecting to login",
                                            session.user_id
                                        );
                                        return login_redirect(&data, private_cookies)
                                            .map_err(WebError::from);
                                    }

                                    // If session is present check if app is in user authorized
                                    // apps. If yes, return auth code and state else redirect to
                                    // consent form.
                                    if let Some(app) =
                                        OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                                            &appstate.pool,
                                            session.user_id,
                                            oauth2client.id,
                                        )
                                        .await?
                                    {
                                        info!(
                                            "OAuth client id {} authorized by user id {}, \
                                            returning auth code",
                                            app.oauth2client_id, session.user_id
                                        );
                                        let private_cookies = private_cookies
                                            .remove(Cookie::from(SIGN_IN_COOKIE_NAME));
                                        let location = generate_auth_code_redirect(
                                            appstate,
                                            data,
                                            session.user_id,
                                        )
                                        .await?;
                                        Ok(redirect_to(location, private_cookies))
                                    } else {
                                        // If authorized app not found redirect to consent form
                                        info!(
                                            "OAuth client id {} not yet authorized by user id {}, \
                                            redirecting to consent form",
                                            oauth2client.id, session.user_id
                                        );
                                        Ok(redirect_to(
                                            format!(
                                                "/consent?{}",
                                                serde_urlencoded::to_string(data).unwrap()
                                            ),
                                            private_cookies,
                                        ))
                                    }
                                }
                            } else {
                                // If session is not present in database, redirect to login.
                                info!(
                                    "Session {} not found, redirecting to login page",
                                    session_cookie.value()
                                );
                                Ok(login_redirect(&data, private_cookies)?)
                            }
                        // If no session cookie provided redirect to login
                        } else {
                            info!("Session cookie not provided, redirecting to login page");
                            Ok(login_redirect(&data, private_cookies)?)
                        };
                    }
                }
            }
            (true, Err(err)) => {
                error!(
                    "OIDC login validation failed for client {}: {err:?}",
                    data.client_id
                );
                error = err;
            }
            (false, _) => {
                error!("OIDC client id {} is disabled", data.client_id);
                error = CoreAuthErrorResponseType::UnauthorizedClient;
            }
        }
    } else {
        error!("OIDC client id {} not found", data.client_id);
        error = CoreAuthErrorResponseType::UnauthorizedClient;
    }

    let mut url = if is_redirect_allowed {
        Url::parse(&data.redirect_uri).map_err(|_| OidcFlowError::InvalidRedirectUri)?
    } else {
        // Don't allow open redirects (DG25-17)
        Settings::url()?
    };
    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("error", error.as_ref());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        }
    };

    Ok(redirect_to(url, private_cookies))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Default)]
pub struct GroupClaims {
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<String>>,
}

impl AdditionalClaims for GroupClaims {}

async fn get_group_claims(pool: &PgPool, user: &User<Id>) -> Result<GroupClaims, OidcFlowError> {
    let groups = user.member_of_names(pool).await?;
    Ok(GroupClaims {
        groups: Some(groups),
    })
}

/// Finish the OAuth2 authorization flow after user consent
///
/// Called by the consent screen once the user allows or denies the request. On approval it
/// redirects back to the client with an authorization code.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/authorize",
    tag = "OAuth2",
    params(
        ("client_id" = String, Query, description = "ID of the OAuth2 client."),
        ("redirect_uri" = String, Query, description = "Redirect URI registered for the client."),
        ("response_type" = String, Query, description = "OAuth2 response type, for example `code`."),
        ("scope" = String, Query, description = "Space-separated list of requested scopes."),
        ("state" = String, Query, description = "Opaque value returned unchanged to the client."),
        ("nonce" = Option<String>, Query, description = "Value bound to the ID token to mitigate replay attacks."),
        ("code_challenge" = Option<String>, Query, description = "PKCE code challenge."),
        ("code_challenge_method" = Option<String>, Query, description = "PKCE code challenge method, for example `S256`."),
        ("prompt" = Option<String>, Query, description = "OpenID `prompt` parameter, for example `consent`."),
        ("allow" = Option<bool>, Query, description = "Set by the consent screen to allow or deny the request."),
    ),
    responses(
        (status = 302, description = "Redirect to the client redirect URI with an authorization code or an error."),
        (status = 400, description = "Invalid authorization request.", body = ApiErrorResponse, example = json!({"msg": "Invalid redirect URI"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to handle authorization request.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn secure_authorization(
    session_info: SessionInfo,
    State(appstate): State<AppState>,
    Query(data): Query<AuthenticationRequest>,
    private_cookies: PrivateCookieJar,
) -> Result<(StatusCode, HeaderMap, PrivateCookieJar), WebError> {
    let error;
    let mut is_redirect_allowed = false;
    if let Some(oauth2client) =
        OAuth2Client::find_by_client_id(&appstate.pool, &data.client_id).await?
    {
        is_redirect_allowed = oauth2client.contains_redirect_url(&data.redirect_uri);
        if data.allow {
            match (
                oauth2client.enabled,
                data.validate_for_client(&oauth2client),
            ) {
                (true, Ok(())) => {
                    if OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                        &appstate.pool,
                        session_info.user.id,
                        oauth2client.id,
                    )
                    .await?
                    .is_none()
                    {
                        let app = OAuth2AuthorizedApp::new(session_info.user.id, oauth2client.id);
                        app.save(&appstate.pool).await?;

                        let mut conn = appstate.pool.begin().await?;
                        new_device_oidc_login_mail(
                            &session_info.user.email,
                            &mut conn,
                            Some(&session_info.session.into()),
                            &oauth2client.name,
                            &session_info.user.username,
                        )
                        .await?;
                    }
                    info!(
                        "User {} allowed login with client {}",
                        session_info.user.username, oauth2client.name
                    );
                    let private_cookies = private_cookies.remove(SIGN_IN_COOKIE_NAME);
                    let location =
                        generate_auth_code_redirect(appstate, data, session_info.user.id).await?;
                    info!(
                        "Redirecting user {} to {location}",
                        session_info.user.username
                    );
                    return Ok(redirect_to(location, private_cookies));
                }
                (true, Err(err)) => {
                    info!(
                        "OIDC login validation failed for user {}, client {}",
                        session_info.user.username, oauth2client.name
                    );
                    error = err;
                }
                (false, _) => {
                    error!("OIDC client id {} is disabled", oauth2client.name);
                    error = CoreAuthErrorResponseType::UnauthorizedClient;
                }
            }
        } else {
            info!(
                "User {} denied OIDC login with app id {}",
                session_info.user.username, data.client_id
            );
            error = CoreAuthErrorResponseType::AccessDenied;
        }
    } else {
        error!(
            "User {} tried to log in with non-existent OIDC client id {}",
            session_info.user.username, data.client_id
        );
        error = CoreAuthErrorResponseType::UnauthorizedClient;
    }

    let mut url = if is_redirect_allowed {
        Url::parse(&data.redirect_uri)?
    } else {
        // Don't allow open redirects (DG25-17)
        Settings::url()?
    };
    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("error", error.as_ref());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        }
    };

    Ok(redirect_to(url, private_cookies))
}

/// https://openid.net/specs/openid-connect-core-1_0.html#TokenRequest
#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: CoreGrantType,
    // grant_type == "authorization_code"
    code: Option<String>,
    redirect_uri: Option<String>,
    // grant_type == "refresh_token"
    refresh_token: Option<String>,
    // scope: Option<String>,
    // Authorization
    client_id: Option<String>,
    client_secret: Option<String>,
    // PKCE
    code_verifier: Option<String>,
}

impl TokenRequest {
    /// Verify Proof Key for Code Exchange (PKCE) https://www.rfc-editor.org/rfc/rfc7636
    fn verify_pkce(&self, code_challenge: Option<&String>) -> bool {
        if let Some(challenge) = code_challenge {
            if let Some(verifier) = &self.code_verifier {
                let pkce_challenge = PkceCodeChallenge::from_code_verifier_sha256(
                    &PkceCodeVerifier::new(verifier.into()),
                );
                pkce_challenge.as_str() == challenge
            } else {
                false
            }
        } else {
            true
        }
    }

    fn authorization_code_flow<T>(
        &self,
        auth_code: &AuthCode<NoId>,
        token: &OAuth2Token,
        claims: StandardClaims<CoreGenderClaim>,
        base_url: &Url,
        secret: T,
        rsa_key: Option<CoreRsaPrivateSigningKey>,
        group_claims: GroupClaims,
    ) -> Result<DefguardTokenResponse, CoreErrorResponseType>
    where
        T: Into<Vec<u8>>,
    {
        // assume self.grant_type == "authorization_code"
        if let (Some(code), Some(redirect_uri)) = (&self.code, &self.redirect_uri) {
            if redirect_uri.trim_end_matches('/') != auth_code.redirect_uri.trim_end_matches('/') {
                error!(
                    "Redirect URIs don't match for client_id {}: {redirect_uri} != {}",
                    self.client_id.as_ref().map_or("Unknown", String::as_str),
                    auth_code.redirect_uri
                );
                return Err(CoreErrorResponseType::UnauthorizedClient);
            }

            if !self.verify_pkce(auth_code.code_challenge.as_ref()) {
                error!(
                    "PKCE verification failed for client id {}",
                    self.client_id.as_ref().map_or("Unknown", String::as_str)
                );
                return Err(CoreErrorResponseType::InvalidRequest);
            }

            let access_token = AccessToken::new(token.access_token.clone());
            // append ID token only when scope contains "openid"
            let id_token = if token.scope.split(' ').any(|scope| scope == "openid") {
                debug!("Scope contains openid, issuing JWT ID token");
                let authorization_code = AuthorizationCode::new(code.into());
                let issue_time = Utc::now();
                let settings = Settings::get_current_settings();
                let timeout = settings.authentication_timeout();
                let expiration = issue_time + timeout;
                let id_token_claims = IdTokenClaims::new(
                    IssuerUrl::from_url(base_url.clone()),
                    vec![Audience::new(auth_code.client_id.clone())],
                    expiration,
                    issue_time,
                    claims,
                    group_claims,
                )
                .set_nonce(auth_code.nonce.clone().map(Nonce::new));

                let id_token = match rsa_key {
                    Some(key) => IdToken::new(
                        id_token_claims,
                        &key,
                        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
                        Some(&access_token),
                        Some(&authorization_code),
                    ),
                    None => IdToken::new(
                        id_token_claims,
                        &CoreHmacKey::new(secret),
                        CoreJwsSigningAlgorithm::HmacSha256,
                        Some(&access_token),
                        Some(&authorization_code),
                    ),
                };
                id_token.ok()
            } else {
                None
            };

            let mut token_response = DefguardTokenResponse::new(
                access_token,
                CoreTokenType::Bearer,
                IdTokenFields::new(id_token, EmptyExtraTokenFields {}),
            );
            token_response.set_refresh_token(Some(RefreshToken::new(token.refresh_token.clone())));
            Ok(token_response)
        } else {
            if self.code.is_none() {
                error!("Request missing code param");
            }
            if self.redirect_uri.is_none() {
                error!("Request missing redirect_uri param");
            }
            Err(CoreErrorResponseType::InvalidRequest)
        }
    }

    fn refresh_token_flow(
        token: &OAuth2Token,
    ) -> StandardTokenResponse<EmptyExtraTokenFields, CoreTokenType> {
        // assume self.grant_type == "refresh_token"
        let access_token = AccessToken::new(token.access_token.clone());
        let refresh_token = RefreshToken::new(token.refresh_token.clone());
        let mut token_response = StandardTokenResponse::new(
            access_token,
            CoreTokenType::Bearer,
            EmptyExtraTokenFields {},
        );
        token_response.set_refresh_token(Some(refresh_token));
        token_response
    }

    async fn oauth2client(&self, pool: &PgPool) -> Option<OAuth2Client<Id>> {
        if let (Some(client_id), Some(client_secret)) =
            (self.client_id.as_ref(), self.client_secret.as_ref())
        {
            OAuth2Client::find_by_auth(pool, client_id, client_secret)
                .await
                .unwrap_or_default()
            // .map_err(|_| CoreErrorResponseType::InvalidClient)
        } else {
            None
        }
    }
}

/// Exchange an authorization code or a refresh token for tokens
///
/// Accepts `application/x-www-form-urlencoded` and supports the `authorization_code` and
/// `refresh_token` grants. The client authenticates with HTTP Basic auth or with
/// `client_id`/`client_secret` in the form body. Implements the
/// [OpenID Connect token endpoint](https://openid.net/specs/openid-connect-core-1_0.html#TokenEndpoint).
#[utoipa::path(
    post,
    path = "/api/v1/oauth/token",
    tag = "OAuth2",
    request_body(
        content = Object,
        content_type = "application/x-www-form-urlencoded",
        description = "`grant_type`, `code` or `refresh_token`, `redirect_uri`, `code_verifier`, and optionally `client_id`/`client_secret`."
    ),
    responses(
        (status = 200, description = "Access token, and an ID token when the `openid` scope was requested.", body = Object, example = json!({
            "access_token": "hR4pV9mK2sT7dQ1xL0nB",
            "token_type": "bearer",
            "refresh_token": "gY6wC3jN8bF5rZ2tM7vK",
            "id_token": "eyJhbGciOiJSUzI1NiJ9..."
        })),
        (status = 400, description = "Invalid grant or invalid request.", body = Object, example = json!({"error": "invalid_grant"})),
        (status = 401, description = "Invalid client credentials.", body = ApiErrorResponse, example = json!({"msg": "Invalid credentials"})),
        (status = 500, description = "Unable to issue token.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn token(
    State(appstate): State<AppState>,
    OAuth2ClientExtractor(oauth2client): OAuth2ClientExtractor,
    Form(form): Form<TokenRequest>,
) -> ApiResult {
    // TODO: cleanup branches
    match form.grant_type {
        CoreGrantType::AuthorizationCode => {
            debug!("Staring authorization_code flow");

            // for logging
            let form_client_id = match &form.client_id {
                Some(id) => id,
                None => "N/A",
            };

            if let Some(code) = &form.code {
                // Look for `AuthCode`. If found, it will be deleted from the database to avoid
                // concurrent requests that might return multiple tokens for the same code.
                // This addresses DG25-24 and conforms to RFC 6749.
                if let Some(auth_code) = AuthCode::find_code(&appstate.pool, code).await? {
                    debug!("Consumed authorization_code {code}, client_id `{form_client_id}`");
                    if let Some(client) = oauth2client.or(form.oauth2client(&appstate.pool).await) {
                        if !client.enabled {
                            error!("OAuth client id `{}` is disabled", client.name);
                            let response = StandardErrorResponse::new(
                                CoreErrorResponseType::UnauthorizedClient,
                                None,
                                None,
                            );
                            return Ok(ApiResponse::json(response, StatusCode::BAD_REQUEST));
                        }

                        if let Some(user) =
                            User::find_by_id(&appstate.pool, auth_code.user_id).await?
                        {
                            if let Some(authorized_app) =
                                OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                                    &appstate.pool,
                                    user.id,
                                    client.id,
                                )
                                .await?
                            {
                                debug!(
                                    "Issuing new token for user {} client {}",
                                    user.username, client.name
                                );
                                // Remove existing tokens in case the same client asks for a new token.
                                OAuth2Token::delete_all_by_authorized_app_id(
                                    &appstate.pool,
                                    authorized_app.id,
                                )
                                .await?;
                                let token = OAuth2Token::new(
                                    authorized_app.id,
                                    auth_code.redirect_uri.clone(),
                                    auth_code.scope.clone(),
                                );
                                let group_claims = if auth_code.scope.contains("groups") {
                                    get_group_claims(&appstate.pool, &user).await?
                                } else {
                                    GroupClaims { groups: None }
                                };
                                let user_claims = UserClaims::from_user(&user, &client, &token);
                                let base_url = Settings::url()?;
                                let openid_key = runtime_openid_key()?;

                                match form.authorization_code_flow(
                                    &auth_code,
                                    &token,
                                    user_claims.into(),
                                    &base_url,
                                    client.client_secret,
                                    openid_key,
                                    group_claims,
                                ) {
                                    Ok(response) => {
                                        token.save(&appstate.pool).await?;
                                        info!(
                                            "Issued new token for user {} client {}",
                                            user.username, client.name
                                        );
                                        return Ok(ApiResponse::json(response, StatusCode::OK));
                                    }
                                    Err(err) => {
                                        error!(
                                            "Error issuing new token for user {} client {}: {err}",
                                            user.username, client.name
                                        );
                                        let response = StandardErrorResponse::new(err, None, None);
                                        return Ok(ApiResponse::json(
                                            response,
                                            StatusCode::BAD_REQUEST,
                                        ));
                                    }
                                }
                            }
                            error!(
                                "Can't issue token - authorized app not found for user {}, client \
                                {}",
                                user.username, client.name
                            );
                        } else {
                            error!("User id {} not found", auth_code.user_id);
                        }
                    } else {
                        error!("OAuth client id `{form_client_id}` not found");
                    }
                } else {
                    error!("OAuth auth code not found");
                }
            } else {
                error!("No code provided in request for client id `{form_client_id}`");
            }
        }
        CoreGrantType::RefreshToken => {
            debug!("Starting refresh_token flow");
            let Some(client) = oauth2client.or(form.oauth2client(&appstate.pool).await) else {
                let err = CoreErrorResponseType::InvalidClient;
                let response = StandardErrorResponse::new(err, None, None);
                return Ok(ApiResponse::json(response, StatusCode::UNAUTHORIZED));
            };

            if !client.enabled {
                error!("OAuth client id `{}` is disabled", client.name);
                let response = StandardErrorResponse::new(
                    CoreErrorResponseType::UnauthorizedClient,
                    None,
                    None,
                );
                return Ok(ApiResponse::json(response, StatusCode::BAD_REQUEST));
            }

            let Some(refresh_token) = form.refresh_token else {
                let err = CoreErrorResponseType::InvalidGrant;
                let response = StandardErrorResponse::new(err, None, None);
                return Ok(ApiResponse::json(response, StatusCode::BAD_REQUEST));
            };
            let Some(mut token) = OAuth2Token::find_by_refresh_token_for_client(
                &appstate.pool,
                &refresh_token,
                client.id,
            )
            .await?
            else {
                let err = CoreErrorResponseType::InvalidGrant;
                let response = StandardErrorResponse::new(err, None, None);
                return Ok(ApiResponse::json(response, StatusCode::BAD_REQUEST));
            };

            token.refresh_and_save(&appstate.pool).await?;
            let response = TokenRequest::refresh_token_flow(&token);
            return Ok(ApiResponse::json(response, StatusCode::OK));
        }
        _ => (), // TODO: Err(CoreErrorResponseType::UnsupportedGrantType),
    }
    let err = CoreErrorResponseType::UnsupportedGrantType;
    let response = StandardErrorResponse::new(err, None, None);
    Ok(ApiResponse::json(response, StatusCode::BAD_REQUEST))
}

/// Get the claims of the authenticated user
///
/// Requires an access token in the `Authorization: Bearer <token>` header. Implements the
/// [OpenID Connect UserInfo endpoint](https://openid.net/specs/openid-connect-core-1_0.html#UserInfo).
#[utoipa::path(
    get,
    path = "/api/v1/oauth/userinfo",
    tag = "OAuth2",
    responses(
        (status = 200, description = "Claims of the authenticated user.", body = Object, example = json!({
            "sub": "admin",
            "name": "Jane Doe",
            "given_name": "Jane",
            "family_name": "Doe",
            "email": "jane@example.com",
            "email_verified": true
        })),
        (status = 401, description = "Access token is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Invalid token"})),
        (status = 500, description = "Unable to get user claims.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn userinfo(State(appstate): State<AppState>, headers: HeaderMap) -> ApiResult {
    let Some(token) = headers.get(AUTHORIZATION).and_then(|value| {
        if let Ok(value) = value.to_str() {
            if value.to_lowercase().starts_with("bearer ") {
                value.get(7..)
            } else {
                None
            }
        } else {
            None
        }
    }) else {
        return Err(WebError::Authorization("Invalid session".into()));
    };

    let Some(oauth2token) = OAuth2Token::find_by_access_token(&appstate.pool, token).await? else {
        return Err(WebError::Authorization("Invalid token".into()));
    };

    let Some(authorized_app) =
        OAuth2AuthorizedApp::find_by_id(&appstate.pool, oauth2token.oauth2authorizedapp_id).await?
    else {
        return Err(WebError::Authorization("Authorized app not found".into()));
    };

    let Some(client) =
        OAuth2Client::find_by_id(&appstate.pool, authorized_app.oauth2client_id).await?
    else {
        return Err(WebError::Authorization("OAuth2 client not found".into()));
    };

    if !client.enabled {
        return Err(WebError::Authorization("OAuth2 client is disabled".into()));
    }

    let Some(user) = User::find_by_id(&appstate.pool, authorized_app.user_id).await? else {
        return Err(WebError::Authorization("User not found".into()));
    };

    let user_claims = UserClaims::from_user(&user, &client, &oauth2token);

    Ok(ApiResponse::json(
        StandardClaims::from(user_claims),
        StatusCode::OK,
    ))
}

// Must be served under /.well-known/openid-configuration
/// Get the OpenID Connect discovery document
///
/// See [OpenID Connect Discovery 1.0](https://openid.net/specs/openid-connect-discovery-1_0.html).
#[utoipa::path(
    get,
    path = "/.well-known/openid-configuration",
    tag = "OAuth2",
    responses(
        (status = 200, description = "Discovery document of this OpenID provider.", body = Object, example = json!({
            "issuer": "https://vpn.example.com/",
            "authorization_endpoint": "https://vpn.example.com/api/v1/oauth/authorize",
            "token_endpoint": "https://vpn.example.com/api/v1/oauth/token",
            "userinfo_endpoint": "https://vpn.example.com/api/v1/oauth/userinfo",
            "jwks_uri": "https://vpn.example.com/api/v1/oauth/discovery/keys",
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["HS256", "RS256"],
            "scopes_supported": ["openid", "profile", "email", "phone", "groups"]
        })),
        (status = 500, description = "Unable to build discovery document.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn openid_configuration() -> ApiResult {
    let url = Settings::url().map_err(|e| OidcFlowError::Url(e.to_string()))?;
    let provider_metadata = CoreProviderMetadata::new(
        IssuerUrl::from_url(url.clone()),
        AuthUrl::from_url(url.join("api/v1/oauth/authorize")?),
        JsonWebKeySetUrl::from_url(url.join("api/v1/oauth/discovery/keys")?),
        vec![ResponseTypes::new(vec![CoreResponseType::Code])],
        vec![CoreSubjectIdentifierType::Public],
        vec![
            CoreJwsSigningAlgorithm::HmacSha256,           // required
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256, // recommended
        ],
        EmptyAdditionalProviderMetadata {},
    )
    .set_token_endpoint(Some(TokenUrl::from_url(url.join("api/v1/oauth/token")?)))
    .set_scopes_supported(Some(vec![
        Scope::new("openid".into()),
        Scope::new("profile".into()),
        Scope::new("email".into()),
        Scope::new("phone".into()),
        Scope::new("groups".into()),
    ]))
    .set_claims_supported(Some(vec![
        CoreClaimName::new("iss".into()),
        CoreClaimName::new("sub".into()),
        CoreClaimName::new("aud".into()),
        CoreClaimName::new("exp".into()),
        CoreClaimName::new("iat".into()),
        CoreClaimName::new("name".into()),
        CoreClaimName::new("given_name".into()),
        CoreClaimName::new("family_name".into()),
        CoreClaimName::new("email".into()),
        CoreClaimName::new("phone_number".into()),
        CoreClaimName::new("groups".into()),
    ]))
    .set_grant_types_supported(Some(vec![
        CoreGrantType::AuthorizationCode,
        CoreGrantType::RefreshToken,
    ]))
    .set_userinfo_endpoint(Some(UserInfoUrl::from_url(
        url.join("api/v1/oauth/userinfo")?,
    )));

    Ok(ApiResponse::json(provider_metadata, StatusCode::OK))
}
