use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{Session, User},
    enterprise::{
        db::openid::{AuthorizedApp, OpenIDClient, OpenIDClientAuth},
        openid_idtoken::IDTokenClaims,
        openid_state::OpenIDRequest,
    },
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
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
                match OpenIDClient::find_enabled_for_client_id(&appstate.pool, &client.client_id)
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

#[derive(Serialize)]
pub struct OpenIDConfiguration {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    scopes_supported: Vec<String>,
    response_types_supported: Vec<String>,
    claims_supported: Vec<String>,
}

#[get("/.well-known/openid-configuration", format = "json")]
pub fn openid_configuration(appstate: &State<AppState>) -> ApiResult {
    let openid_config = OpenIDConfiguration {
        issuer: appstate.config.url.clone(),
        authorization_endpoint: format!("{}/openid/authorize", appstate.config.url),
        token_endpoint: format!("{}/api/openid/token", appstate.config.url),
        scopes_supported: vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "phone".into(),
        ],
        response_types_supported: vec!["code".into()],
        claims_supported: vec![
            "iss".into(),
            "sub".into(),
            "aud".into(), // TODO: add to JWT? https://openid.net/specs/openid-connect-core-1_0.html
            "exp".into(),
            "iat".into(),
            "given_name".into(),
            "family_name".into(),
            "email".into(),
            "email_verified".into(),
            "phone".into(),
            "phone_verified".into(),
            "nonce".into(),
        ],
    };
    Ok(ApiResponse {
        json: json!(openid_config),
        status: Status::Ok,
    })
}
