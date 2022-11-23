use crate::{
    appstate::AppState,
    auth::SESSION_TIMEOUT,
    db::User,
    enterprise::db::{authorization_code::AuthorizationCode, oauth::OAuth2Token, OAuth2Client},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use chrono::{Duration, Utc};
use openidconnect::{
    core::{
        CoreClaimName, CoreErrorResponseType, CoreHmacKey, CoreIdToken, CoreIdTokenClaims,
        CoreIdTokenFields, CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
        CoreSubjectIdentifierType, CoreTokenType,
    },
    url::Url,
    AccessToken, Audience, AuthUrl, EmptyAdditionalClaims, EmptyAdditionalProviderMetadata,
    EmptyExtraTokenFields, IssuerUrl, JsonWebKeySetUrl, Nonce, ResponseTypes, Scope,
    StandardClaims, StandardErrorResponse, StandardTokenResponse, SubjectIdentifier, TokenUrl,
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
    // prompt:
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

/// Authorization Endpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#AuthorizationEndpoint
#[get("/authorize?<data..>")]
pub async fn authentication(
    appstate: &State<AppState>,
    data: AuthenticationRequest<'_>,
) -> Result<Redirect, OriWebError> {
    // TODO: PKCE https://www.rfc-editor.org/rfc/rfc7636
    info!("XXX {}", data.client_id);
    let err = match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
        Some(oauth2client) => match data.validate_for_client(&oauth2client) {
            Ok(_) => {
                let mut code = AuthorizationCode::new(
                    oauth2client.user_id,
                    data.client_id.into(),
                    data.redirect_uri.into(),
                    data.scope.into(), // FIXME: is this needed?
                    data.nonce.map(str::to_owned),
                );
                code.save(&appstate.pool).await?;
                // FIXME: missing a proper struct from `openidconnect`; check CoreResponseType::Code
                return Ok(Redirect::found(format!(
                    "{}?code={}&state={}",
                    data.redirect_uri, code.code, data.state
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

#[derive(FromForm)]
pub struct IDTokenRequest<'r> {
    grant_type: &'r str,
    code: &'r str,
    redirect_uri: &'r str,
}

/// Token Endpoint
/// https://openid.net/specs/openid-connect-core-1_0.html#TokenEndpoint
#[post("/token", data = "<form>")]
pub async fn id_token(form: Form<IDTokenRequest<'_>>, appstate: &State<AppState>) -> ApiResult {
    let err;

    // Currentl only "authorization_code" is supported for `grant_type`
    if form.grant_type != "authorization_code" {
        err = CoreErrorResponseType::UnsupportedGrantType;
    } else {
        info!("Verifying authorization code: {}", form.code);
        if let Some(auth_code) = AuthorizationCode::find_code(&appstate.pool, form.code).await? {
            if form.redirect_uri != auth_code.redirect_uri {
                err = CoreErrorResponseType::UnauthorizedClient;
            } else {
                info!("Checking session for user: {}", auth_code.user_id);
                if let Some(user) = User::find_by_id(&appstate.pool, auth_code.user_id).await? {
                    // Create user claims based on scope
                    // let user_claims = IDTokenClaims::get_user_claims(user, &auth_code.scope);
                    let oauth2client = OAuth2Client::find_enabled_for_client_id(
                        &appstate.pool,
                        &auth_code.client_id,
                    )
                    .await?
                    .unwrap();

                    let token =
                        OAuth2Token::new(auth_code.redirect_uri.clone(), auth_code.scope.clone());
                    // token.save(&appstate.pool).await?;

                    let access_token = AccessToken::new(token.access_token.clone());
                    let authorization_code =
                        openidconnect::AuthorizationCode::new(auth_code.code.clone());
                    let issue_time = Utc::now();
                    let expiration = issue_time + Duration::seconds(SESSION_TIMEOUT as i64);
                    let std_claims = StandardClaims::new(SubjectIdentifier::new(user.username));
                    let base_url = Url::parse(&appstate.config.url).unwrap();
                    let claims = CoreIdTokenClaims::new(
                        IssuerUrl::from_url(base_url),
                        vec![Audience::new(auth_code.client_id.clone())],
                        expiration,
                        issue_time,
                        std_claims,
                        EmptyAdditionalClaims {},
                    )
                    .set_nonce(auth_code.nonce.clone().map(Nonce::new));
                    let signing_key = CoreHmacKey::new(oauth2client.client_secret);
                    let id_token = CoreIdToken::new(
                        claims,
                        &signing_key,
                        CoreJwsSigningAlgorithm::HmacSha256,
                        Some(&access_token),
                        Some(&authorization_code),
                    )
                    .unwrap();
                    let response = StandardTokenResponse::new(
                        access_token,
                        CoreTokenType::Bearer,
                        CoreIdTokenFields::new(Some(id_token), EmptyExtraTokenFields {}),
                    );
                    info!("{:#?}", response);

                    // Remove client authorization code from database
                    auth_code
                        .delete(&appstate.pool)
                        .await
                        .map_err(|_| OriWebError::ObjectNotFound("Failed to remove code".into()))?;

                    return Ok(ApiResponse {
                        json: json!(response),
                        status: Status::Ok,
                    });
                } else {
                    err = CoreErrorResponseType::UnauthorizedClient;
                }
            }
        } else {
            err = CoreErrorResponseType::InvalidGrant;
        }
    }

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
