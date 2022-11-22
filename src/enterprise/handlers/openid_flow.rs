use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{Session, User},
    enterprise::{
        db::{
            oauth::AuthorizationCode,
            openid::{AuthorizedApp, OpenIDClientAuth},
            OAuth2Client,
        },
        openid_idtoken::IDTokenClaims,
        openid_state::OpenIDRequest,
    },
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use openidconnect::{
    core::{
        CoreClaimName, CoreErrorResponseType, CoreJwsSigningAlgorithm, CoreProviderMetadata,
        CoreResponseType, CoreSubjectIdentifierType,
    },
    url::Url,
    AuthUrl, EmptyAdditionalProviderMetadata, IssuerUrl, JsonWebKeySetUrl, ResponseTypes, Scope,
    StandardErrorResponse, TokenUrl,
};
use rocket::{
    form::{Form, Lenient},
    http::Status,
    response::Redirect,
    serde::json::{serde_json::json, Json},
    State,
};

// Check if app is authorized, return 200 or 404
#[post("/verify", format = "json", data = "<data>")]
pub async fn check_authorized(
    session: Session,
    data: Json<OpenIDRequest>,
    appstate: &State<AppState>,
) -> ApiResult {
    let status = match AuthorizedApp::find_by_user_and_client_id(
        &appstate.pool,
        session.user_id,
        &data.client_id,
    )
    .await?
    {
        Some(_app) => Status::Ok,
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

// https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
// see https://docs.rs/oauth2/latest/src/oauth2/lib.rs.html
#[derive(Debug, FromForm)]
pub struct AuthenticationRequest<'r> {
    scope: &'r str,
    response_type: &'r str,
    client_id: &'r str,
    client_secret: Option<&'r str>,
    redirect_uri: &'r str,
    state: &'r str,
    // response_mode: Option<&'r str>,
    // nonce: Option<&'r str>,
    // display:
    // prompt:
    // max_age:
    // ui_locales:
    // id_token_hint:
    // login_hint:
    // acr_values
}

impl<'r> AuthenticationRequest<'r> {
    // TODO: return CoreErrorResponseType
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

#[get("/authorize?<data..>")]
pub async fn authentication(
    appstate: &State<AppState>,
    data: AuthenticationRequest<'_>,
) -> Result<Redirect, OriWebError> {
    // TODO: PKCE https://www.rfc-editor.org/rfc/rfc7636
    match OAuth2Client::find_by_client_id(&appstate.pool, data.client_id).await? {
        Some(oauth2client) => match data.validate_for_client(&oauth2client) {
            Ok(_) => {
                // FIXME: missing a proper struct from `openidconnect`; check CoreResponseType::Code
                let mut code = AuthorizationCode::new(
                    oauth2client.user_id,
                    data.client_id.into(),
                    data.redirect_uri.into(),
                    data.scope.into(), // FIXME: is this needed?
                );
                code.save(&appstate.pool).await?;
                Ok(Redirect::found(format!(
                    "{}?code={}&state={}",
                    data.redirect_uri, code.code, data.state
                )))
            }
            Err(err) => {
                let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
                Ok(Redirect::found(format!(
                    "{}?{}",
                    data.redirect_uri,
                    serde_qs::to_string(&response).unwrap()
                )))
            }
        },
        None => {
            let err = CoreErrorResponseType::InvalidClient;
            let response = StandardErrorResponse::<CoreErrorResponseType>::new(err, None, None);
            Ok(Redirect::found(format!(
                "{}?{}",
                data.redirect_uri,
                serde_qs::to_string(&response).unwrap()
            )))
        }
    }
}

// Login endpoint redirect with authorization code on success, or error if something failed
// https://openid.net/specs/openid-connect-core-1_0.html#ImplicitAuthorizationEndpoint
// Generate PKCE code challenge, store in the database
// and return 302 redirect for given URL with state and code
#[post("/authorize?<data..>")]
pub async fn authentication_request(
    session: SessionInfo,
    data: Lenient<OpenIDRequest>,
    appstate: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let openid_request = data.into_inner();
    debug!("Verifying client: {}", openid_request.client_id);
    openid_request
        .create_code(
            &appstate.pool,
            &session.user.username,
            session.user.id.unwrap(),
        )
        .await
}

#[derive(FromForm)]
pub struct IDTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
}

// Create token with scopes based on client
#[post("/token", data = "<form>")]
pub async fn id_token(form: Form<IDTokenRequest>, appstate: &State<AppState>) -> ApiResult {
    debug!("Verifying authorization code: {}", &form.code);
    if let Some(client) = OpenIDClientAuth::find_by_code(&appstate.pool, &form.code).await? {
        // Check user session and create id token
        debug!("Checking session for user: {}", &client.user);
        if let Some(user) = User::find_by_username(&appstate.pool, &client.user).await? {
            // Create user claims based on scope
            let user_claims = IDTokenClaims::get_user_claims(user, &client.scope);
            let secret =
                match OAuth2Client::find_enabled_for_client_id(&appstate.pool, &client.client_id)
                    .await?
                {
                    Some(client) => client.client_secret,
                    None => {
                        return Err(OriWebError::ObjectNotFound(
                            "Failed to find client secret corresponding to id".to_string(),
                        ));
                    }
                };
            debug!("Creating ID Token for {}", &client.user);
            let token = IDTokenClaims::new(
                user_claims.username.clone(),
                client.client_id.clone(),
                client.nonce.clone(),
                user_claims,
            )
            .to_jwt(&secret)
            .map_err(|_| OriWebError::Authorization("Failed to create ID token".to_string()))?;
            info!("ID Token for user {} created", &client.user);

            // Remove client authorization code from database
            // FIXME: this used to the first statement in this function -- check if it is valid here
            client
                .delete(&appstate.pool)
                .await
                .map_err(|_| OriWebError::ObjectNotFound("Failed to remove client".into()))?;

            Ok(ApiResponse {
                json: json!({ "id_token": token }),
                status: Status::Ok,
            })
        } else {
            Ok(ApiResponse {
                json: json!({
                    "error":
                    "failed to get user session",
                }),
                status: Status::BadRequest,
            })
        }
    } else {
        Ok(ApiResponse {
            json: json!({"error": "failed to authorize client"}),
            status: Status::BadRequest,
        })
    }
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
        vec![CoreJwsSigningAlgorithm::HmacSha256], // match with auth::Claims.encode()
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
