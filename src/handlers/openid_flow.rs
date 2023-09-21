use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Query, State},
    http::{header::LOCATION, request::Parts, HeaderMap, HeaderValue, StatusCode},
    Form,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::Utc;
use openidconnect::{
    core::{
        CoreAuthErrorResponseType, CoreClaimName, CoreErrorResponseType, CoreGenderClaim,
        CoreGrantType, CoreHmacKey, CoreIdToken, CoreIdTokenClaims, CoreIdTokenFields,
        CoreJsonWebKeySet, CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
        CoreRsaPrivateSigningKey, CoreSubjectIdentifierType, CoreTokenResponse, CoreTokenType,
    },
    url::Url,
    AccessToken, Audience, AuthUrl, AuthorizationCode, EmptyAdditionalClaims,
    EmptyAdditionalProviderMetadata, EmptyExtraTokenFields, EndUserEmail, EndUserFamilyName,
    EndUserGivenName, EndUserName, EndUserPhoneNumber, EndUserUsername, IssuerUrl,
    JsonWebKeySetUrl, LocalizedClaim, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    PrivateSigningKey, RefreshToken, ResponseTypes, Scope, StandardClaims, StandardErrorResponse,
    StandardTokenResponse, SubjectIdentifier, TokenUrl, UserInfoUrl,
};
use secrecy::ExposeSecret;
use serde::{
    de::{Deserialize, Deserializer, Error as DeError, Unexpected, Visitor},
    ser::{Serialize, Serializer},
};
use serde_json::json;
use tower_cookies::{
    cookie::{
        time::{Duration, OffsetDateTime},
        SameSite,
    },
    Cookie, Cookies, Key,
};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AccessUserInfo, SessionInfo, SESSION_TIMEOUT},
    db::{
        models::{auth_code::AuthCode, oauth2client::OAuth2Client},
        DbPool, OAuth2AuthorizedApp, OAuth2Token, Session, User,
    },
    error::WebError,
    handlers::SIGN_IN_COOKIE_NAME,
};

/// https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims
impl From<&User> for StandardClaims<CoreGenderClaim> {
    fn from(user: &User) -> StandardClaims<CoreGenderClaim> {
        let mut name = LocalizedClaim::new();
        name.insert(None, EndUserName::new(user.name()));
        let mut given_name = LocalizedClaim::new();
        given_name.insert(None, EndUserGivenName::new(user.first_name.clone()));
        let mut given_name = LocalizedClaim::new();
        given_name.insert(None, EndUserGivenName::new(user.first_name.clone()));
        let mut family_name = LocalizedClaim::new();
        family_name.insert(None, EndUserFamilyName::new(user.last_name.clone()));
        let email = EndUserEmail::new(user.email.clone());
        let phone_number = user.phone.clone().map(EndUserPhoneNumber::new);
        let preferred_username = EndUserUsername::new(user.username.clone());

        StandardClaims::new(SubjectIdentifier::new(user.username.clone()))
            .set_name(Some(name))
            .set_given_name(Some(given_name))
            .set_family_name(Some(family_name))
            .set_email(Some(email))
            .set_phone_number(phone_number)
            .set_preferred_username(Some(preferred_username))
    }
}

pub async fn discovery_keys(State(appstate): State<AppState>) -> ApiResult {
    let mut keys = Vec::new();
    if let Some(openid_key) = appstate.config.openid_key() {
        keys.push(openid_key.as_verification_key());
    };

    Ok(ApiResponse {
        json: json!(CoreJsonWebKeySet::new(keys)),
        status: StatusCode::OK,
    })
}

/// Provide `OAuth2Client` when Basic Authorization header contains `client_id` and `client_secret`.
#[async_trait]
impl<S> FromRequestParts<S> for OAuth2Client
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        if let Some(basic_auth) = parts.headers.get("authorization").and_then(|value| {
            if let Ok(value) = value.to_str() {
                if value.starts_with("Basic ") {
                    value.get(6..)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            if let Ok(decoded) = BASE64_STANDARD.decode(basic_auth) {
                if let Ok(auth_pair) = String::from_utf8(decoded) {
                    if let Some((client_id, client_secret)) = auth_pair.split_once(':') {
                        if let Ok(Some(oauth2client)) =
                            OAuth2Client::find_by_auth(&appstate.pool, client_id, client_secret)
                                .await
                        {
                            return Ok(oauth2client);
                        }
                    }
                }
            }
        }
        Err(WebError::Authorization("Invalid credentials".into()))
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

impl<'de> Visitor<'de> for FieldResponseTypesVisitor {
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
    state: Option<String>,
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
        oauth2client: &OAuth2Client,
    ) -> Result<(), CoreAuthErrorResponseType> {
        // check scope: it is valid if any requested scope exists in the `oauth2client`
        if self
            .scope
            .split(' ')
            .all(|scope| !oauth2client.scope.contains(&scope.to_owned()))
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

        // check `redirect_uri` matches client config (ignoring trailing slashes)
        let parsed_redirect_uris: Vec<String> = oauth2client
            .redirect_uri
            .clone()
            .into_iter()
            .map(|uri| uri.trim_end_matches('/').into())
            .collect();
        if self
            .redirect_uri
            .split(' ')
            .map(|uri| uri.trim_end_matches('/'))
            .all(|uri| !parsed_redirect_uris.contains(&uri.to_owned()))
        {
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
        if self.code_challenge.is_some() && self.code_challenge_method != Some("S256".to_string()) {
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
    user_id: Option<i64>,
) -> Result<String, WebError> {
    let mut url =
        Url::parse(&data.redirect_uri).map_err(|_| WebError::Http(StatusCode::BAD_REQUEST))?;
    let mut auth_code = AuthCode::new(
        user_id.unwrap(),
        data.client_id,
        data.redirect_uri,
        data.scope,
        data.nonce,
        data.code_challenge,
    );
    auth_code.save(&appstate.pool).await?;

    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("code", auth_code.code.as_str());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        };
    };

    Ok(url.to_string())
}

/// Helper function to return redirection with status code 302.
fn redirect_to<T: AsRef<str>>(uri: T) -> (StatusCode, HeaderMap) {
    let mut headers = HeaderMap::new();
    headers.insert(
        LOCATION,
        HeaderValue::try_from(uri.as_ref()).expect("URI isn't a valid header value"),
    );

    (StatusCode::FOUND, headers)
}

/// Helper function to redirect unauthorized user to login page
/// and store information about OpenID authorize url in cookie to redirect later
async fn login_redirect(
    appstate: AppState,
    data: &AuthenticationRequest,
    cookies: Cookies,
) -> Result<(StatusCode, HeaderMap), WebError> {
    let base_url = appstate.config.url.join("api/v1/oauth/authorize").unwrap();
    let expires = OffsetDateTime::now_utc()
        .checked_add(Duration::minutes(10))
        .ok_or(WebError::Http(StatusCode::INTERNAL_SERVER_ERROR))?;
    let cookie = Cookie::build(
        SIGN_IN_COOKIE_NAME,
        format!("{base_url}?{}", serde_urlencoded::to_string(data).unwrap()),
    )
    .secure(true)
    .same_site(SameSite::Strict)
    .http_only(true)
    .expires(expires)
    .finish();
    let key = Key::from(appstate.config.secret_key.expose_secret().as_bytes());
    let private_cookies = cookies.private(&key);
    private_cookies.add(cookie);
    Ok(redirect_to("/login"))
}

/// Authorization Endpoint
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
pub async fn authorization(
    State(appstate): State<AppState>,
    Query(data): Query<AuthenticationRequest>,
    cookies: Cookies,
) -> Result<(StatusCode, HeaderMap), WebError> {
    let error;
    if let Some(oauth2client) =
        OAuth2Client::find_by_client_id(&appstate.pool, &data.client_id).await?
    {
        match data.validate_for_client(&oauth2client) {
            Ok(()) => {
                match &data.prompt {
                    Some(s) if s == "consent" => {
                        info!(
                            "Redirecting user to consent form - client id {}",
                            data.client_id
                        );
                        return Ok(redirect_to(format!(
                            "/consent?{}",
                            serde_urlencoded::to_string(data).unwrap(),
                        )));
                    }
                    Some(s) if s == "none" => {
                        error!("'none' prompt in client id {} request", data.client_id);
                        error = CoreAuthErrorResponseType::LoginRequired;
                    }
                    _ => {
                        return if let Some(session_cookie) = cookies.get("defguard_session") {
                            if let Ok(Some(session)) =
                                Session::find_by_id(&appstate.pool, session_cookie.value()).await
                            {
                                // If session expired return login
                                if session.expired() {
                                    info!("Session {} for user id {} has expired, redirecting to login", session.id, session.user_id);
                                    let _result = session.delete(&appstate.pool).await;
                                    login_redirect(appstate, &data, cookies).await
                                } else {
                                    // If session is present check if app is in user authorized apps.
                                    // If yes return auth code and state else redirect to consent form.
                                    if let Some(app) =
                                        OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                                            &appstate.pool,
                                            session.user_id,
                                            oauth2client.id.unwrap(),
                                        )
                                        .await?
                                    {
                                        info!(
                                            "OAuth client id {} authorized by user id {}, returning auth code",
                                            app.oauth2client_id, session.user_id
                                        );
                                        let key = Key::from(
                                            appstate.config.secret_key.expose_secret().as_bytes(),
                                        );
                                        let private_cookies = cookies.private(&key);
                                        private_cookies.remove(Cookie::named(SIGN_IN_COOKIE_NAME));
                                        let location = generate_auth_code_redirect(
                                            appstate,
                                            data,
                                            Some(session.user_id),
                                        )
                                        .await?;
                                        Ok(redirect_to(location))
                                    } else {
                                        // If authorized app not found redirect to consent form
                                        info!(
                                            "OAuth client id {} not yet authorized by user id {}, redirecting to consent form",
                                            oauth2client.id.unwrap(), session.user_id
                                        );
                                        Ok(redirect_to(format!(
                                            "/consent?{}",
                                            serde_urlencoded::to_string(data).unwrap()
                                        )))
                                    }
                                }
                            } else {
                                // If session is not present in db redirect to login
                                info!(
                                    "Session {} not found, redirecting to login page",
                                    session_cookie.value()
                                );
                                login_redirect(appstate, &data, cookies).await
                            }

                        // If no session cookie provided redirect to login
                        } else {
                            info!("Session cookie not provided, redirecting to login page");
                            login_redirect(appstate, &data, cookies).await
                        };
                    }
                }
            }
            Err(err) => {
                error!(
                    "OIDC login validation failed for client {}: {err:?}",
                    data.client_id
                );
                error = err;
            }
        }
    } else {
        error!("OIDC client id {} not found", data.client_id);
        error = CoreAuthErrorResponseType::UnauthorizedClient;
    }

    let mut url =
        Url::parse(&data.redirect_uri).map_err(|_| WebError::Http(StatusCode::BAD_REQUEST))?;

    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("error", error.as_ref());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        };
    };

    Ok(redirect_to(url))
}

/// Login Authorization Endpoint redirect with authorization code
pub async fn secure_authorization(
    session_info: SessionInfo,
    State(appstate): State<AppState>,
    Query(data): Query<AuthenticationRequest>,
    cookies: Cookies,
) -> Result<(StatusCode, HeaderMap), WebError> {
    let mut url =
        Url::parse(&data.redirect_uri).map_err(|_| WebError::Http(StatusCode::BAD_REQUEST))?;
    let error;
    if data.allow {
        if let Some(oauth2client) =
            OAuth2Client::find_by_client_id(&appstate.pool, &data.client_id).await?
        {
            match data.validate_for_client(&oauth2client) {
                Ok(()) => {
                    if OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                        &appstate.pool,
                        session_info.user.id.unwrap(),
                        oauth2client.id.unwrap(),
                    )
                    .await?
                    .is_none()
                    {
                        let mut app = OAuth2AuthorizedApp::new(
                            session_info.user.id.unwrap(),
                            oauth2client.id.unwrap(),
                        );
                        app.save(&appstate.pool).await?;
                    }
                    info!(
                        "User {} allowed login with client {}",
                        session_info.user.username, oauth2client.name
                    );
                    let key = Key::from(appstate.config.secret_key.expose_secret().as_bytes());
                    let private_cookies = cookies.private(&key);
                    if let Some(cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                        cookies.remove(cookie.clone());
                    };
                    let location =
                        generate_auth_code_redirect(appstate, data, session_info.user.id).await?;
                    info!(
                        "Redirecting user {} to {location}",
                        session_info.user.username
                    );
                    return Ok(redirect_to(location));
                }
                Err(err) => {
                    info!(
                        "OIDC login validation failed for user {}, client {}",
                        session_info.user.username, oauth2client.name
                    );
                    error = err;
                }
            }
        } else {
            error!(
                "User {} tried to log in with non-existent OIDC client id {}",
                session_info.user.username, data.client_id
            );
            error = CoreAuthErrorResponseType::UnauthorizedClient;
        }
    } else {
        info!(
            "User {} denied OIDC login with app id {}",
            session_info.user.username, data.client_id
        );
        error = CoreAuthErrorResponseType::AccessDenied;
    }

    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("error", error.as_ref());
        if let Some(state) = data.state {
            query_pairs.append_pair("state", &state);
        };
    };

    Ok(redirect_to(url))
}

/// https://openid.net/specs/openid-connect-core-1_0.html#TokenRequest
#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: String,
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
        auth_code: &AuthCode,
        token: &OAuth2Token,
        claims: StandardClaims<CoreGenderClaim>,
        base_url: &Url,
        secret: T,
        rsa_key: Option<CoreRsaPrivateSigningKey>,
    ) -> Result<CoreTokenResponse, CoreErrorResponseType>
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
                let expiration = issue_time + chrono::Duration::seconds(SESSION_TIMEOUT as i64);
                let id_token_claims = CoreIdTokenClaims::new(
                    IssuerUrl::from_url(base_url.clone()),
                    vec![Audience::new(auth_code.client_id.clone())],
                    expiration,
                    issue_time,
                    claims,
                    EmptyAdditionalClaims {},
                )
                .set_nonce(auth_code.nonce.clone().map(Nonce::new));
                let id_token = match rsa_key {
                    Some(key) => CoreIdToken::new(
                        id_token_claims,
                        &key,
                        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
                        Some(&access_token),
                        Some(&authorization_code),
                    ),
                    None => CoreIdToken::new(
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

            let mut token_response = CoreTokenResponse::new(
                access_token,
                CoreTokenType::Bearer,
                CoreIdTokenFields::new(id_token, EmptyExtraTokenFields {}),
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

    async fn oauth2client(&self, pool: &DbPool) -> Option<OAuth2Client> {
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

/// Token Endpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#TokenEndpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#RefreshTokens
pub async fn token(
    State(appstate): State<AppState>,
    oauth2client: Option<OAuth2Client>,
    Form(form): Form<TokenRequest>,
) -> ApiResult {
    // TODO: cleanup branches
    match form.grant_type.as_str() {
        "authorization_code" => {
            debug!("Staring authorization_code flow");

            // for logging
            let form_client_id = match &form.client_id {
                Some(id) => id.clone(),
                None => String::from("N/A"),
            };

            if let Some(code) = &form.code {
                if let Some(stored_auth_code) = AuthCode::find_code(&appstate.pool, code).await? {
                    // copy data before removing used token
                    let auth_code = stored_auth_code.clone();
                    // remove authorization_code from DB so it cannot be reused
                    debug!(
                        "Removing used authorization_code {code}, client_id `{}`",
                        form_client_id
                    );
                    stored_auth_code.consume(&appstate.pool).await?;
                    if let Some(client) = oauth2client.or(form.oauth2client(&appstate.pool).await) {
                        if let Some(user) =
                            User::find_by_id(&appstate.pool, auth_code.user_id).await?
                        {
                            if let Some(authorized_app) =
                                OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                                    &appstate.pool,
                                    user.id.unwrap(),
                                    client.id.unwrap(),
                                )
                                .await?
                            {
                                debug!(
                                    "Issuing new token for user {} client {}",
                                    user.username, client.name
                                );
                                // Remove existing token in case same client asks for new token
                                if let Some(token) = OAuth2Token::find_by_authorized_app_id(
                                    &appstate.pool,
                                    authorized_app.id.unwrap(),
                                )
                                .await?
                                {
                                    token.delete(&appstate.pool).await?;
                                }
                                let token = OAuth2Token::new(
                                    authorized_app.id.unwrap(),
                                    auth_code.redirect_uri.clone(),
                                    auth_code.scope.clone(),
                                );
                                match form.authorization_code_flow(
                                    &auth_code,
                                    &token,
                                    (&user).into(),
                                    &appstate.config.url,
                                    client.client_secret,
                                    appstate.config.openid_key(),
                                ) {
                                    Ok(response) => {
                                        token.save(&appstate.pool).await?;
                                        info!(
                                            "Issued new token for user {} client {}",
                                            user.username, client.name
                                        );
                                        return Ok(ApiResponse {
                                            json: json!(response),
                                            status: StatusCode::OK,
                                        });
                                    }
                                    Err(err) => {
                                        error!(
                                            "Error issuing new token for user {} client {}: {}",
                                            user.username, client.name, err
                                        );
                                        let response =
                                            StandardErrorResponse::<CoreErrorResponseType>::new(
                                                err, None, None,
                                            );
                                        return Ok(ApiResponse {
                                            json: json!(response),
                                            status: StatusCode::BAD_REQUEST,
                                        });
                                    }
                                }
                            }
                            error!("Can't issue token - authorized app not found for user {}, client {}", user.username, client.name);
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
                error!("No code provided in request for client id `{form_client_id}`",);
            }
        }
        "refresh_token" => {
            debug!("Starting refresh_token flow");
            if let Some(refresh_token) = form.refresh_token {
                if let Ok(Some(mut token)) =
                    OAuth2Token::find_refresh_token(&appstate.pool, &refresh_token).await
                {
                    token.refresh_and_save(&appstate.pool).await?;
                    let response = TokenRequest::refresh_token_flow(&token);
                    token.save(&appstate.pool).await?;
                    return Ok(ApiResponse {
                        json: json!(response),
                        status: StatusCode::OK,
                    });
                }
            }
        }
        _ => (), // TODO: Err(CoreErrorResponseType::UnsupportedGrantType),
    }
    let err = CoreErrorResponseType::UnsupportedGrantType;
    let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
    Ok(ApiResponse {
        json: json!(response),
        status: StatusCode::BAD_REQUEST,
    })
}

/// https://openid.net/specs/openid-connect-core-1_0.html#UserInfo
pub async fn userinfo(user_info: AccessUserInfo) -> ApiResult {
    let userclaims = StandardClaims::<CoreGenderClaim>::from(&user_info.0);
    Ok(ApiResponse {
        json: json!(userclaims),
        status: StatusCode::OK,
    })
}

// Must be served under /.well-known/openid-configuration
pub async fn openid_configuration(State(appstate): State<AppState>) -> ApiResult {
    let provider_metadata = CoreProviderMetadata::new(
        IssuerUrl::from_url(appstate.config.url.clone()),
        AuthUrl::from_url(appstate.config.url.join("api/v1/oauth/authorize").unwrap()),
        JsonWebKeySetUrl::from_url(
            appstate
                .config
                .url
                .join("api/v1/oauth/discovery/keys")
                .unwrap(),
        ),
        vec![ResponseTypes::new(vec![CoreResponseType::Code])],
        vec![CoreSubjectIdentifierType::Public],
        vec![
            CoreJwsSigningAlgorithm::HmacSha256,           // required
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256, // recommended
        ],
        EmptyAdditionalProviderMetadata {},
    )
    .set_token_endpoint(Some(TokenUrl::from_url(
        appstate.config.url.join("api/v1/oauth/token").unwrap(),
    )))
    .set_scopes_supported(Some(vec![
        Scope::new("openid".into()),
        Scope::new("profile".into()),
        Scope::new("email".into()),
        Scope::new("phone".into()),
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
    ]))
    .set_grant_types_supported(Some(vec![
        CoreGrantType::AuthorizationCode,
        CoreGrantType::RefreshToken,
    ]))
    .set_userinfo_endpoint(Some(UserInfoUrl::from_url(
        appstate.config.url.join("api/v1/oauth/userinfo").unwrap(),
    )));

    Ok(ApiResponse {
        json: json!(provider_metadata),
        status: StatusCode::OK,
    })
}
