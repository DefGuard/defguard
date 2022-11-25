use crate::{
    appstate::AppState,
    auth::SESSION_TIMEOUT,
    enterprise::db::{auth_code::AuthCode, oauth2token::OAuth2Token, OAuth2Client},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use chrono::{Duration, Utc};
use openidconnect::{
    core::{
        CoreClaimName, CoreErrorResponseType, CoreHmacKey, CoreIdToken, CoreIdTokenClaims,
        CoreIdTokenFields, CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
        CoreSubjectIdentifierType, CoreTokenResponse, CoreTokenType,
    },
    url::Url,
    AccessToken, Audience, AuthUrl, AuthorizationCode, EmptyAdditionalClaims,
    EmptyAdditionalProviderMetadata, EmptyExtraTokenFields, IssuerUrl, JsonWebKeySetUrl, Nonce,
    RefreshToken, ResponseTypes, Scope, StandardClaims, StandardErrorResponse,
    StandardTokenResponse, SubjectIdentifier, TokenUrl,
};
use rocket::{form::Form, http::Status, response::Redirect, serde::json::serde_json::json, State};

// Check if app is authorized, return 200 or 404
// #[post("/verify", format = "json", data = "<data>")]
// pub async fn check_authorized(
//     session: Session,
//     data: Json<OpenIDRequest>,
//     appstate: &State<AppState>,
// ) -> ApiResult {
//     let status = match AuthorizedApp::find_by_user_and_client_id(
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

// https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
// see https://docs.rs/oauth2/latest/src/oauth2/lib.rs.html
#[derive(FromForm)]
pub struct AuthenticationRequest<'r> {
    scope: &'r str,
    response_type: &'r str,
    client_id: &'r str,
    client_secret: Option<&'r str>,
    redirect_uri: &'r str,
    state: &'r str,
    // response_mode: Option<&'r str>,
    nonce: Option<&'r str>,
    // display:
    // prompt: Option<&'r str>,
    // max_age:
    // ui_locales:
    // id_token_hint:
    // login_hint:
    // acr_values
}

impl<'r> AuthenticationRequest<'r> {
    fn validate_for_client(
        &self,
        oauth2client: &OAuth2Client,
    ) -> Result<(), CoreErrorResponseType> {
        // check scope: for now it is valid is any requested scope exists in the `oauth2client`
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

        // check `client_secret`
        if let Some(secret) = self.client_secret {
            if oauth2client.client_secret != secret {
                return Err(CoreErrorResponseType::InvalidGrant);
            }
        }

        // check `redirect_uri`
        // TODO: allow multiple uris in `oauth2client`
        if self.redirect_uri != oauth2client.redirect_uri {
            return Err(CoreErrorResponseType::InvalidGrant);
        }

        Ok(())
    }
}

// https://openid.net/specs/openid-connect-core-1_0.html#AuthResponse
// FIXME: missing a proper struct from `openidconnect`; check CoreResponseType::Code
#[derive(Deserialize, Serialize)]
pub struct AuthenticationResponse {
    pub code: String,
    pub state: String,
}

/// Authorization Endpoint
// https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
#[get("/authorize?<data..>")]
pub async fn authentication(
    appstate: &State<AppState>,
    data: AuthenticationRequest<'_>,
) -> Result<Redirect, OriWebError> {
    // TODO: PKCE https://www.rfc-editor.org/rfc/rfc7636
    let err = match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
        Some(oauth2client) => match data.validate_for_client(&oauth2client) {
            Ok(_) => {
                let mut code = AuthCode::new(
                    oauth2client.user_id,
                    data.client_id.into(),
                    data.redirect_uri.into(),
                    data.scope.into(),
                    data.nonce.map(str::to_owned),
                );
                code.save(&appstate.pool).await?;
                let response = AuthenticationResponse {
                    code: code.code,
                    state: data.state.into(),
                };
                return Ok(Redirect::found(format!(
                    "{}?{}",
                    data.redirect_uri,
                    serde_qs::to_string(&response).unwrap()
                )));
            }
            Err(err) => err,
        },
        None => CoreErrorResponseType::InvalidClient,
    };

    let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
    Ok(Redirect::found(format!(
        "{}?{}",
        data.redirect_uri,
        serde_qs::to_string(&response).unwrap()
    )))
}

// Login endpoint redirect with authorization code on success, or error if something failed
// https://openid.net/specs/openid-connect-core-1_0.html#ImplicitAuthorizationEndpoint
// Generate PKCE code challenge, store in the database
// and return 302 redirect for given URL with state and code
// #[post("/authorize?<data..>")]
// pub async fn authentication_request(
//     session: SessionInfo,
//     data: Lenient<OpenIDRequest>,
//     appstate: &State<AppState>,
// ) -> Result<Redirect, Redirect> {
//     let openid_request = data.into_inner();
//     debug!("Verifying client: {}", openid_request.client_id);
//     openid_request
//         .create_code(
//             &appstate.pool,
//             &session.user.username,
//             session.user.id.unwrap(),
//         )
//         .await
// }

/// https://openid.net/specs/openid-connect-core-1_0.html#TokenRequest
#[derive(FromForm)]
pub struct TokenRequest<'r> {
    grant_type: &'r str,
    // grant_type == "authorization_code"
    code: Option<&'r str>,
    redirect_uri: Option<&'r str>,
    // grant_type == "refresh_token"
    // client_id: Option<&'r str>,
    // client_secret: Option<&'r str>,
    refresh_token: Option<&'r str>,
    // scope: Option<&'r str>,
}

impl<'r> TokenRequest<'r> {
    fn authorization_code_flow<T>(
        &self,
        auth_code: &AuthCode,
        token: &OAuth2Token,
        claims_subject: String,
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

            let access_token = AccessToken::new(token.access_token.clone());
            let authorization_code = AuthorizationCode::new(code.into());
            let issue_time = Utc::now();
            let expiration = issue_time + Duration::seconds(SESSION_TIMEOUT as i64);
            let std_claims = StandardClaims::new(SubjectIdentifier::new(claims_subject));
            let claims = CoreIdTokenClaims::new(
                IssuerUrl::from_url(base_url),
                vec![Audience::new(auth_code.client_id.clone())],
                expiration,
                issue_time,
                std_claims,
                EmptyAdditionalClaims {},
            )
            .set_nonce(auth_code.nonce.clone().map(Nonce::new));
            let signing_key = CoreHmacKey::new(secret);
            let refresh_token = RefreshToken::new(token.refresh_token.clone());
            match CoreIdToken::new(
                claims,
                &signing_key,
                CoreJwsSigningAlgorithm::HmacSha256,
                Some(&access_token),
                Some(&authorization_code),
            ) {
                Ok(id_token) => {
                    let mut token_response = CoreTokenResponse::new(
                        access_token,
                        CoreTokenType::Bearer,
                        CoreIdTokenFields::new(Some(id_token), EmptyExtraTokenFields {}),
                    );
                    token_response.set_refresh_token(Some(refresh_token));
                    Ok(token_response)
                }
                Err(err) => Err(CoreErrorResponseType::Extension(err.to_string())),
            }
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
}

/// Token Endpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#TokenEndpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#RefreshTokens
#[post("/token", format = "form", data = "<form>")]
pub async fn id_token(form: Form<TokenRequest<'_>>, appstate: &State<AppState>) -> ApiResult {
    // TODO: implement basic authorization

    // TODO: cleanup branches
    match form.grant_type {
        "authorization_code" => {
            if let Some(code) = form.code {
                if let Some(auth_code) = AuthCode::find_code(&appstate.pool, code).await? {
                    if let Some(oauth2client) = OAuth2Client::find_enabled_for_client_id(
                        &appstate.pool,
                        &auth_code.client_id,
                    )
                    .await?
                    {
                        let token = OAuth2Token::new(
                            auth_code.redirect_uri.clone(),
                            auth_code.scope.clone(),
                        );
                        let base_url = Url::parse(&appstate.config.url).unwrap();
                        match form.authorization_code_flow(
                            &auth_code,
                            &token,
                            "username".into(), // FIXME: get real username
                            base_url,
                            oauth2client.client_secret,
                        ) {
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
        }
        "refresh_token" => {
            if let Some(refresh_token) = form.refresh_token {
                if let Some(mut token) =
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

// Must be served under /.well-known/openid-configuration
#[get("/openid-configuration", format = "json")]
pub fn openid_configuration(appstate: &State<AppState>) -> ApiResult {
    let base_url = Url::parse(&appstate.config.url).unwrap();
    let provider_metadata = CoreProviderMetadata::new(
        IssuerUrl::from_url(base_url.clone()),
        AuthUrl::from_url(base_url.join("api/v1/openid/authorize").unwrap()),
        JsonWebKeySetUrl::from_url(base_url.join("api/v1/oauth/discovery/keys").unwrap()),
        vec![ResponseTypes::new(vec![CoreResponseType::Code])],
        vec![CoreSubjectIdentifierType::Public],
        vec![
            CoreJwsSigningAlgorithm::HmacSha256, // required
        ], // match with auth::Claims.encode()
        EmptyAdditionalProviderMetadata {},
    )
    .set_token_endpoint(Some(TokenUrl::from_url(
        base_url.join("api/v1/openid/token").unwrap(),
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
        CoreClaimName::new("given_name".into()),
        CoreClaimName::new("family_name".into()),
        CoreClaimName::new("email".into()),
        CoreClaimName::new("email_verified".into()),
        CoreClaimName::new("phone".into()),
        CoreClaimName::new("phone_verified".into()),
        CoreClaimName::new("nonce".into()),
    ]));

    Ok(ApiResponse {
        json: json!(provider_metadata),
        status: Status::Ok,
    })
}
