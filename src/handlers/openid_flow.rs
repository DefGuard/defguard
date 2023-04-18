use crate::{
    appstate::AppState,
    auth::{SessionInfo, SESSION_TIMEOUT},
    db::{
        models::{auth_code::AuthCode, oauth2client::OAuth2Client},
        OAuth2AuthorizedApp, OAuth2Token,
    },
    db::{DbPool, Session, User},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use chrono::{Duration, Utc};
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
use rocket::{
    form::{self, Form, FromFormField, ValueField},
    http::{Cookie, CookieJar, Status},
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    serde::json::serde_json::json,
    State,
};

use std::ops::{Deref, DerefMut};

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

#[get("/discovery/keys")]
pub async fn discovery_keys(appstate: &State<AppState>) -> ApiResult {
    let mut keys = Vec::new();
    if let Some(openid_key) = appstate.config.openid_key() {
        keys.push(openid_key.as_verification_key());
    };

    Ok(ApiResponse {
        json: json!(CoreJsonWebKeySet::new(keys)),
        status: Status::Ok,
    })
}

/// Provide `OAuth2Client` when Basic Authorization header contains `client_id` and `client_secret`.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for OAuth2Client {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let state = request
            .rocket()
            .state::<AppState>()
            .expect("Missing AppState");
        if let Some(basic_auth) = request
            .headers()
            .get_one("Authorization")
            .and_then(|value| {
                if value.starts_with("Basic ") {
                    value.get(6..)
                } else {
                    None
                }
            })
        {
            if let Ok(decoded) = base64::decode(basic_auth) {
                if let Ok(auth_pair) = String::from_utf8(decoded) {
                    if let Some((client_id, client_secret)) = auth_pair.split_once(':') {
                        if let Ok(Some(oauth2client)) =
                            OAuth2Client::find_by_auth(&state.pool, client_id, client_secret).await
                        {
                            return Outcome::Success(oauth2client);
                        }
                    }
                }
            }
            Outcome::Failure((
                Status::Unauthorized,
                OriWebError::Authorization("Invalid credentials".into()),
            ))
        } else {
            Outcome::Forward(())
        }
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

impl serde::ser::Serialize for FieldResponseTypes {
    // serialize to a string with values separated by space
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let types: Vec<&str> = self.iter().map(CoreResponseType::as_ref).collect();
        serializer.serialize_str(&types.join(" "))
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for FieldResponseTypes {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let mut response_types = FieldResponseTypes(Vec::new());
        for value in field.value.split(' ') {
            match value {
                "code" => response_types.push(CoreResponseType::Code),
                "id_token" => response_types.push(CoreResponseType::IdToken),
                "token" => response_types.push(CoreResponseType::Token),
                _ => Err(form::Error::validation("invalid value for response type"))?,
            }
        }
        Ok(response_types)
    }
}

/// Authentication Request
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
#[derive(FromForm, Serialize)]
pub struct AuthenticationRequest<'r> {
    scope: &'r str,
    response_type: FieldResponseTypes,
    client_id: &'r str,
    // client_secret: Option<&'r str>,
    redirect_uri: &'r str,
    state: &'r str,
    // response_mode: Option<&'r str>,
    nonce: Option<&'r str>,
    // display: Option<&'r str>,
    prompt: Option<&'r str>,
    // max_age: Option<&'r str>,
    // ui_locales: Option<&'r str>,
    // id_token_hint: Option<&'r str>,
    // login_hint: Option<&'r str>,
    // acr_values: Option<&'r str>,
    // PKCE
    code_challenge: Option<&'r str>,
    code_challenge_method: Option<&'r str>,
}

impl<'r> AuthenticationRequest<'r> {
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

        // check `redirect_uri`
        if self
            .redirect_uri
            .split(' ')
            .all(|uri| !oauth2client.redirect_uri.contains(&uri.to_owned()))
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
        if self.code_challenge.is_some() && self.code_challenge_method != Some("S256") {
            error!(
                "Invalid PKCE method: {}, only S256 supported",
                self.code_challenge_method.unwrap_or("None"),
            );
            return Err(CoreAuthErrorResponseType::InvalidRequest);
        }

        info!("Validation succeeded for client {}", oauth2client.name);
        Ok(())
    }
}

/// Helper function which creates redirect Uri with authorization code
async fn generate_auth_code_redirect(
    appstate: &State<AppState>,
    data: &AuthenticationRequest<'_>,
    user_id: Option<i64>,
) -> Result<String, OriWebError> {
    let mut url =
        Url::parse(data.redirect_uri).map_err(|_| OriWebError::Http(Status::BadRequest))?;
    let mut auth_code = AuthCode::new(
        user_id.unwrap(),
        data.client_id.into(),
        data.redirect_uri.into(),
        data.scope.into(),
        data.nonce.map(str::to_owned),
        data.code_challenge.map(str::to_owned),
    );
    auth_code.save(&appstate.pool).await?;
    url.query_pairs_mut()
        .append_pair("code", auth_code.code.as_str())
        .append_pair("state", data.state);
    Ok(url.to_string())
}

/// Helper function to redirect unauthorized user to login page
/// and store information about OpenID authorize url in cookie to redirect later
async fn login_redirect(
    appstate: &State<AppState>,
    data: &AuthenticationRequest<'_>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, OriWebError> {
    let base_url = appstate.config.url.join("api/v1/oauth/authorize").unwrap();
    cookies.add(Cookie::new(
        "known_sign_in",
        format!(
            "{}?{}",
            base_url,
            serde_urlencoded::to_string(data).unwrap()
        ),
    ));
    Ok(Redirect::found("/login".to_string()))
}

/// Authorization Endpoint
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
#[get("/authorize?<data..>")]
pub async fn authorization(
    appstate: &State<AppState>,
    data: AuthenticationRequest<'_>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, OriWebError> {
    let error;
    match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
        Some(oauth2client) => match data.validate_for_client(&oauth2client) {
            Ok(()) => match data.prompt {
                Some("consent") => {
                    info!(
                        "Redirecting user to consent form - client id {}",
                        data.client_id
                    );
                    return Ok(Redirect::found(format!(
                        "/consent?{}",
                        serde_urlencoded::to_string(data).unwrap()
                    )));
                }
                Some("none") => {
                    error!("'none' prompt in client id {} request", data.client_id);
                    error = CoreAuthErrorResponseType::LoginRequired;
                }
                _ => {
                    if let Some(session_cookie) = cookies.get("defguard_session") {
                        match Session::find_by_id(&appstate.pool, session_cookie.value()).await {
                            Ok(Some(session)) => {
                                // If session expired return login
                                if session.expired() {
                                    info!("Session {} for user id {} has expired, redirecting to login", session.id, session.user_id);
                                    let _result = session.delete(&appstate.pool).await;
                                    return login_redirect(appstate, &data, cookies).await;
                                } else {
                                    // If session is present check if app is in user authorized apps.
                                    // If yes return auth code and state else redirect to consent form.
                                    match OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                                        &appstate.pool,
                                        session.user_id,
                                        oauth2client.id.unwrap(),
                                    )
                                    .await?
                                    {
                                        Some(app) => {
                                            info!("OAuth client id {} authorized by user id {}, returning auth code", app.oauth2client_id, session.user_id);
                                            cookies.remove(Cookie::named("known_sign_in"));
                                            let location = generate_auth_code_redirect(
                                                appstate,
                                                &data,
                                                Some(session.user_id),
                                            )
                                            .await?;
                                            return Ok(Redirect::found(location));
                                        }
                                        // If authorized app not found redirect to consent form
                                        None => {
                                            info!("OAuth client id {} not yet authorized by user id {}, redirecting to consent form", oauth2client.id.unwrap(), session.user_id);
                                            return Ok(Redirect::found(format!(
                                                "/consent?{}",
                                                serde_urlencoded::to_string(data).unwrap()
                                            )));
                                        }
                                    }
                                }
                            }
                            // If session not present in db redirect to login
                            _ => {
                                info!(
                                    "Session {} not found, redirecting to login page",
                                    session_cookie.value()
                                );
                                return login_redirect(appstate, &data, cookies).await;
                            }
                        }

                    // If no session cookie provided redirect to login
                    } else {
                        info!("Session cookie not provided, redirecting to login page");
                        return login_redirect(appstate, &data, cookies).await;
                    }
                }
            },
            Err(err) => {
                error!(
                    "OIDC login validation failed for client {}: {:?}",
                    data.client_id, err
                );
                error = err;
            }
        },
        None => {
            error!("OIDC client id {} not found", data.client_id);
            error = CoreAuthErrorResponseType::UnauthorizedClient;
        }
    }

    let mut url =
        Url::parse(data.redirect_uri).map_err(|_| OriWebError::Http(Status::BadRequest))?;
    url.query_pairs_mut().append_pair("error", error.as_ref());
    Ok(Redirect::found(url.to_string()))
}

/// Login Authorization Endpoint redirect with authorization code
#[post("/authorize?<allow>&<data..>")]
pub async fn secure_authorization(
    session_info: SessionInfo,
    appstate: &State<AppState>,
    allow: bool,
    data: AuthenticationRequest<'_>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, OriWebError> {
    let mut url =
        Url::parse(data.redirect_uri).map_err(|_| OriWebError::Http(Status::BadRequest))?;
    let error;
    if allow {
        match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
            Some(oauth2client) => match data.validate_for_client(&oauth2client) {
                Ok(()) => {
                    match OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
                        &appstate.pool,
                        session_info.user.id.unwrap(),
                        oauth2client.id.unwrap(),
                    )
                    .await?
                    {
                        Some(_) => {}
                        None => {
                            let mut app = OAuth2AuthorizedApp::new(
                                session_info.user.id.unwrap(),
                                oauth2client.id.unwrap(),
                            );
                            app.save(&appstate.pool).await?;
                        }
                    }
                    info!(
                        "User {} allowed login with client {}",
                        session_info.user.username, oauth2client.name
                    );
                    if let Some(cookie) = cookies.get("known_sign_in") {
                        cookies.remove(cookie.to_owned());
                    };
                    let location =
                        generate_auth_code_redirect(appstate, &data, session_info.user.id).await?;
                    info!(
                        "Redirecting user {} to {}",
                        session_info.user.username, location
                    );
                    return Ok(Redirect::found(location));
                }
                Err(err) => {
                    info!(
                        "OIDC login validation failed for user {}, client {}",
                        session_info.user.username, oauth2client.name
                    );
                    error = err;
                }
            },
            None => {
                error!(
                    "User {} tried to log in with non-existent OIDC client id {}",
                    session_info.user.username, data.client_id
                );
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

    url.query_pairs_mut().append_pair("error", error.as_ref());
    Ok(Redirect::found(url.to_string()))
}

/// https://openid.net/specs/openid-connect-core-1_0.html#TokenRequest
#[derive(FromForm)]
pub struct TokenRequest<'r> {
    grant_type: &'r str,
    // grant_type == "authorization_code"
    code: Option<&'r str>,
    redirect_uri: Option<&'r str>,
    // grant_type == "refresh_token"
    refresh_token: Option<&'r str>,
    // scope: Option<&'r str>,
    // Authorization
    client_id: Option<&'r str>,
    client_secret: Option<&'r str>,
    // PKCE
    code_verifier: Option<&'r str>,
}

impl<'r> TokenRequest<'r> {
    /// Verify Proof Key for Code Exchange (PKCE) https://www.rfc-editor.org/rfc/rfc7636
    fn verify_pkce(&self, code_challenge: Option<&String>) -> bool {
        if let Some(challenge) = code_challenge {
            if let Some(verifier) = self.code_verifier {
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
        if let (Some(code), Some(redirect_uri)) = (self.code, self.redirect_uri) {
            if redirect_uri != auth_code.redirect_uri {
                error!(
                    "Redirect URIs don't match for client_id {}: {} != {}",
                    self.client_id.unwrap_or("Unknown"),
                    redirect_uri,
                    auth_code.redirect_uri
                );
                return Err(CoreErrorResponseType::UnauthorizedClient);
            }

            if !self.verify_pkce(auth_code.code_challenge.as_ref()) {
                error!(
                    "PKCE verification failed for client id {}",
                    self.client_id.unwrap_or("Unknown")
                );
                return Err(CoreErrorResponseType::InvalidRequest);
            }

            let access_token = AccessToken::new(token.access_token.clone());
            // append ID token only when scope contains "openid"
            let id_token = if token.scope.split(' ').any(|scope| scope == "openid") {
                debug!("Scope contains openid, issuing JWT ID token");
                let authorization_code = AuthorizationCode::new(code.into());
                let issue_time = Utc::now();
                let expiration = issue_time + Duration::seconds(SESSION_TIMEOUT as i64);
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
        &self,
        token: &OAuth2Token,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, CoreTokenType>, CoreErrorResponseType>
    {
        // assume self.grant_type == "refresh_token"
        let access_token = AccessToken::new(token.access_token.clone());
        let refresh_token = RefreshToken::new(token.refresh_token.clone());
        let mut token_response = StandardTokenResponse::new(
            access_token,
            CoreTokenType::Bearer,
            EmptyExtraTokenFields {},
        );
        token_response.set_refresh_token(Some(refresh_token));
        Ok(token_response)
    }

    async fn oauth2client(&self, pool: &DbPool) -> Option<OAuth2Client> {
        if let (Some(client_id), Some(client_secret)) = (self.client_id, self.client_secret) {
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
#[post("/token", format = "form", data = "<form>")]
pub async fn token(
    form: Form<TokenRequest<'_>>,
    appstate: &State<AppState>,
    oauth2client: Option<OAuth2Client>,
) -> ApiResult {
    // TODO: cleanup branches
    match form.grant_type {
        "authorization_code" => {
            debug!("Staring authorization_code flow");
            if let Some(code) = form.code {
                if let Some(auth_code) = AuthCode::find_code(&appstate.pool, code).await? {
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
                                            status: Status::Ok,
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
                                            status: Status::BadRequest,
                                        });
                                    }
                                }
                            } else {
                                error!("Can't issue token - authorized app not found for user {}, client {}", user.username, client.name);
                            }
                        } else {
                            error!("User id {} not found", auth_code.user_id);
                        }
                    } else {
                        error!("OAuth client id {} not found", form.client_id.unwrap_or(""));
                    }
                } else {
                    error!("OAuth auth code not found");
                }
            } else {
                error!(
                    "No code provided in request for client id {}",
                    form.client_id.unwrap_or("")
                );
            }
        }
        "refresh_token" => {
            debug!("Starting refresh_token flow");
            if let Some(refresh_token) = form.refresh_token {
                if let Ok(Some(mut token)) =
                    OAuth2Token::find_refresh_token(&appstate.pool, refresh_token).await
                {
                    token.refresh_and_save(&appstate.pool).await?;
                    match form.refresh_token_flow(&token) {
                        Ok(response) => {
                            token.save(&appstate.pool).await?;
                            return Ok(ApiResponse {
                                json: json!(response),
                                status: Status::Ok,
                            });
                        }
                        Err(err) => {
                            let response = StandardErrorResponse::<CoreErrorResponseType>::new(
                                err, None, None,
                            );
                            return Ok(ApiResponse {
                                json: json!(response),
                                status: Status::BadRequest,
                            });
                        }
                    }
                }
            }
        }
        _ => (), // TODO: Err(CoreErrorResponseType::UnsupportedGrantType),
    }
    let err = CoreErrorResponseType::UnsupportedGrantType;
    let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
    Ok(ApiResponse {
        json: json!(response),
        status: Status::BadRequest,
    })
}

/// https://openid.net/specs/openid-connect-core-1_0.html#UserInfo
#[get("/userinfo", format = "json")]
pub fn userinfo(session_info: SessionInfo) -> ApiResult {
    let userclaims = StandardClaims::<CoreGenderClaim>::from(&session_info.user);
    Ok(ApiResponse {
        json: json!(userclaims),
        status: Status::Ok,
    })
}

// Must be served under /.well-known/openid-configuration
#[get("/openid-configuration", format = "json")]
pub fn openid_configuration(appstate: &State<AppState>) -> ApiResult {
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
        status: Status::Ok,
    })
}
