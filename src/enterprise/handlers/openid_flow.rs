use crate::{
    appstate::AppState,
    auth::{SessionInfo, SESSION_TIMEOUT},
    db::{DbPool, User},
    enterprise::db::{AuthCode, OAuth2Client, OAuth2Token},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use chrono::{Duration, Utc};
use openidconnect::{
    core::{
        CoreClaimName, CoreErrorResponseType, CoreGenderClaim, CoreGrantType, CoreHmacKey,
        CoreIdToken, CoreIdTokenClaims, CoreIdTokenFields, CoreJsonWebKeySet,
        CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType, CoreSubjectIdentifierType,
        CoreTokenResponse, CoreTokenType,
    },
    url::Url,
    AccessToken, Audience, AuthUrl, AuthorizationCode, EmptyAdditionalClaims,
    EmptyAdditionalProviderMetadata, EmptyExtraTokenFields, EndUserEmail, EndUserFamilyName,
    EndUserGivenName, EndUserName, EndUserPhoneNumber, IssuerUrl, JsonWebKeySetUrl, LocalizedClaim,
    Nonce, PkceCodeChallenge, PkceCodeVerifier, RefreshToken, ResponseTypes, Scope, StandardClaims,
    StandardErrorResponse, StandardTokenResponse, SubjectIdentifier, TokenUrl, UserInfoUrl,
};
use rocket::{
    form::Form,
    http::Status,
    request::{FromRequest, Outcome},
    response::Redirect,
    serde::json::serde_json::json,
    Request, State,
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

        StandardClaims::new(SubjectIdentifier::new(user.username.clone()))
            .set_name(Some(name))
            .set_given_name(Some(given_name))
            .set_family_name(Some(family_name))
            .set_email(Some(email))
            .set_phone_number(phone_number)
    }
}

#[get("/discovery/keys")]
pub async fn discovery_keys() -> ApiResult {
    // let public_key = RsaPublicKey::from_public_key_pem(PUBLIC_KEY).unwrap();
    // let jwks = CoreJsonWebKeySet::new(vec![CoreJsonWebKey::new_rsa(
    //     public_key.n().to_bytes_be(),
    //     public_key.e().to_bytes_be(),
    //     None,
    // )]);
    let jwks = CoreJsonWebKeySet::new(Vec::new());

    Ok(ApiResponse {
        json: json!(jwks),
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

// Check if app is authorized, return 200 or 404
// #[post("/verify", format = "json", data = "<data>")]
// pub async fn check_authorized(
//     session: Session,
//     data: Json<OpenIDRequest>,
//     appstate: &State<AppState>,
// ) -> ApiResult {
//     let status = match OAuth2Client::find_by_user_and_client_id(
//         &appstate.pool,
//         session.user_id,
//         &data.client_id,
//     )
//     .await?
//     {
//         Some(_app) => Status::Ok,
//         None => Status::NotFound,
//     };
//     Ok(ApiResponse {
//         json: json!({}),
//         status,
//     })
// }

/// Authentication Request
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
#[derive(FromForm, Serialize)]
pub struct AuthenticationRequest<'r> {
    scope: &'r str,
    response_type: &'r str,
    client_id: &'r str,
    // client_secret: Option<&'r str>,
    redirect_uri: &'r str,
    state: &'r str,
    // response_mode: Option<&'r str>,
    nonce: Option<&'r str>,
    // display: Option<&'r str>,
    // prompt: Option<&'r str>,
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
    ) -> Result<(), CoreErrorResponseType> {
        // check scope: it is valid if any requested scope exists in the `oauth2client`
        if self
            .scope
            .split(' ')
            .all(|scope| !oauth2client.scope.contains(&scope.to_owned()))
        {
            return Err(CoreErrorResponseType::InvalidScope);
        }

        // currenly we support only "code" for `response_type`
        if self.response_type != "code" {
            return Err(CoreErrorResponseType::InvalidRequest);
        }

        // assume `client_id` is the same here and in `oauth2client`

        // check `redirect_uri`
        if self
            .redirect_uri
            .split(' ')
            .all(|uri| !oauth2client.redirect_uri.contains(&uri.to_owned()))
        {
            return Err(CoreErrorResponseType::InvalidGrant);
        }

        // check PKCE; currently, only SHA-256 method is supported
        // TODO: support `plain` which is the default if not specified
        if self.code_challenge.is_some() && self.code_challenge_method != Some("S256") {
            return Err(CoreErrorResponseType::InvalidRequest);
        }

        Ok(())
    }
}

// https://openid.net/specs/openid-connect-core-1_0.html#AuthResponse
// FIXME: missing a proper struct from `openidconnect`; check CoreResponseType::Code
#[derive(Deserialize, Serialize)]
pub struct AuthenticationResponse<'r> {
    pub code: &'r str,
    pub state: &'r str,
}

/// Authorization Endpoint
/// See https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
#[get("/authorize?<data..>")]
pub async fn authorization(
    appstate: &State<AppState>,
    data: AuthenticationRequest<'_>,
) -> Result<Redirect, OriWebError> {
    let query = match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
        Some(oauth2client) => match data.validate_for_client(&oauth2client) {
            Ok(()) => {
                return Ok(Redirect::found(format!(
                    "/consent?{}",
                    serde_urlencoded::to_string(data).unwrap()
                )));
            }
            Err(err) => {
                let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
                serde_qs::to_string(&response)?
            }
        },
        None => {
            let err = CoreErrorResponseType::InvalidClient;
            let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
            serde_qs::to_string(&response)?
        }
    };

    Ok(Redirect::found(format!("{}?{}", data.redirect_uri, query)))
}

/// Login Authorization Endpoint redirect with authorization code
#[post("/authorize?<allow>&<data..>")]
pub async fn secure_authorization(
    _session_info: SessionInfo,
    appstate: &State<AppState>,
    allow: bool,
    data: AuthenticationRequest<'_>,
) -> Result<Redirect, OriWebError> {
    let query = if allow {
        match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
            Some(oauth2client) => match data.validate_for_client(&oauth2client) {
                Ok(()) => {
                    let mut auth_code = AuthCode::new(
                        oauth2client.user_id,
                        data.client_id.into(),
                        data.redirect_uri.into(),
                        data.scope.into(),
                        data.nonce.map(str::to_owned),
                        data.code_challenge.map(str::to_owned),
                    );
                    auth_code.save(&appstate.pool).await?;
                    let response = AuthenticationResponse {
                        code: auth_code.code.as_str(),
                        state: data.state,
                    };
                    serde_qs::to_string(&response)?
                }
                Err(err) => {
                    let response =
                        StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
                    serde_qs::to_string(&response)?
                }
            },
            None => {
                let err = CoreErrorResponseType::InvalidClient;
                let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
                serde_qs::to_string(&response)?
            }
        }
    } else {
        let err = CoreErrorResponseType::UnauthorizedClient;
        let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
        serde_qs::to_string(&response)?
    };
    Ok(Redirect::found(format!("{}?{}", data.redirect_uri, query)))
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
    fn authorization_code_flow<T>(
        &self,
        auth_code: &AuthCode,
        token: &OAuth2Token,
        claims: StandardClaims<CoreGenderClaim>,
        base_url: Url,
        secret: T,
    ) -> Result<CoreTokenResponse, CoreErrorResponseType>
    where
        T: Into<Vec<u8>>,
    {
        // assume self.grant_type == "authorization_code"

        if let (Some(code), Some(redirect_uri)) = (self.code, self.redirect_uri) {
            if redirect_uri != auth_code.redirect_uri {
                return Err(CoreErrorResponseType::UnauthorizedClient);
            }

            // Proof Key for Code Exchange (PKCE) https://www.rfc-editor.org/rfc/rfc7636
            if let Some(code_challenge) = &auth_code.code_challenge {
                match self.code_verifier {
                    Some(code_verifier) => {
                        let pkce_code_challenge = PkceCodeChallenge::from_code_verifier_sha256(
                            &PkceCodeVerifier::new(code_verifier.into()),
                        );
                        if pkce_code_challenge.as_str() != code_challenge {
                            return Err(CoreErrorResponseType::InvalidRequest);
                        }
                    }
                    None => return Err(CoreErrorResponseType::InvalidRequest),
                }
            }

            let access_token = AccessToken::new(token.access_token.clone());
            // append ID token only when scope contains "openid"
            let id_token = if token.scope.split(' ').any(|scope| scope == "openid") {
                let authorization_code = AuthorizationCode::new(code.into());
                let issue_time = Utc::now();
                let expiration = issue_time + Duration::seconds(SESSION_TIMEOUT as i64);
                let id_token_claims = CoreIdTokenClaims::new(
                    IssuerUrl::from_url(base_url),
                    vec![Audience::new(auth_code.client_id.clone())],
                    expiration,
                    issue_time,
                    claims,
                    EmptyAdditionalClaims {},
                )
                .set_nonce(auth_code.nonce.clone().map(Nonce::new));
                let signing_key = CoreHmacKey::new(secret);
                CoreIdToken::new(
                    id_token_claims,
                    &signing_key,
                    CoreJwsSigningAlgorithm::HmacSha256,
                    Some(&access_token),
                    Some(&authorization_code),
                )
                .ok()
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
pub async fn id_token(
    form: Form<TokenRequest<'_>>,
    appstate: &State<AppState>,
    oauth2client: Option<OAuth2Client>,
) -> ApiResult {
    // TODO: cleanup branches
    match form.grant_type {
        "authorization_code" => {
            if let Some(code) = form.code {
                if let Some(auth_code) = AuthCode::find_code(&appstate.pool, code).await? {
                    if let Some(client) = oauth2client.or(form.oauth2client(&appstate.pool).await) {
                        if let Some(user) =
                            User::find_by_id(&appstate.pool, auth_code.user_id).await?
                        {
                            let token = OAuth2Token::new(
                                user.id.unwrap(),
                                auth_code.redirect_uri.clone(),
                                auth_code.scope.clone(),
                            );
                            let base_url = Url::parse(&appstate.config.url).unwrap();
                            match form.authorization_code_flow(
                                &auth_code,
                                &token,
                                (&user).into(),
                                base_url,
                                client.client_secret,
                            ) {
                                Ok(response) => {
                                    token.save(&appstate.pool).await?;
                                    return Ok(ApiResponse {
                                        json: json!(response),
                                        status: Status::Ok,
                                    });
                                }
                                Err(err) => {
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
                        }
                    }
                }
            }
        }
        "refresh_token" => {
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
    info!("Userinfo for user {}", session_info.user.username);
    let userclaims = StandardClaims::<CoreGenderClaim>::from(&session_info.user);
    Ok(ApiResponse {
        json: json!(userclaims),
        status: Status::Ok,
    })
}

// Must be served under /.well-known/openid-configuration
#[get("/openid-configuration", format = "json")]
pub fn openid_configuration(appstate: &State<AppState>) -> ApiResult {
    let base_url = Url::parse(&appstate.config.url).unwrap();
    let provider_metadata = CoreProviderMetadata::new(
        IssuerUrl::from_url(base_url.clone()),
        AuthUrl::from_url(base_url.join("api/v1/oauth/authorize").unwrap()),
        JsonWebKeySetUrl::from_url(base_url.join("api/v1/oauth/discovery/keys").unwrap()),
        vec![ResponseTypes::new(vec![CoreResponseType::Code])],
        vec![CoreSubjectIdentifierType::Public],
        vec![
            CoreJwsSigningAlgorithm::HmacSha256, // required
        ],
        EmptyAdditionalProviderMetadata {},
    )
    .set_token_endpoint(Some(TokenUrl::from_url(
        base_url.join("api/v1/oauth/token").unwrap(),
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
        base_url.join("api/v1/oauth/userinfo").unwrap(),
    )));

    Ok(ApiResponse {
        json: json!(provider_metadata),
        status: Status::Ok,
    })
}
