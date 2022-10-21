use crate::{
    db::DbPool,
    enterprise::oauth_db::{AuthorizationCode, OAuth2Client, OAuth2Token},
    oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse, WebError},
};
use oxide_auth::{
    code_grant::{
        accesstoken::Request as TokenRequest, authorization::Request as AuthRequest,
        extensions::Pkce,
    },
    endpoint::{OAuthError, OwnerConsent, QueryParameter, Scopes, Solicitation, Template},
    frontends::simple::extensions::{AccessTokenAddon, AddonResult, AuthorizationAddon},
    primitives::{
        grant::{Extensions, Grant},
        issuer::{IssuedToken, RefreshedToken},
        registrar::{BoundClient, ClientUrl, ExactUrl, PreGrant, RegisteredUrl, RegistrarError},
        scope::Scope,
    },
};
use oxide_auth_async::{
    code_grant::{
        access_token::Extension as AccessTokenExtension,
        authorization::Extension as AuthorizationExtension,
    },
    endpoint::{Endpoint, Extension, OwnerSolicitor},
    primitives::{Authorizer, Issuer, Registrar},
};
use rocket::{http, Response};
use std::borrow::Cow;

// Must implement Clone for flows.
#[derive(Clone)]
pub struct OAuthState {
    pool: DbPool,
    pub decision: bool,
    pub allow: bool,
}

impl OAuthState {
    pub async fn new(pool: DbPool) -> Self {
        // FIXME: Hard-coded client. It should be removed once client management has been implemented.
        let client = OAuth2Client {
            user: "dummy".into(),
            client_id: "LocalClient".into(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost:3000/".into(),
            scope: "default-scope".into(),
        };
        // FIXME: use result
        let _result = client.save(&pool).await;

        OAuthState {
            pool,
            decision: false,
            allow: false,
        }
    }
}

#[async_trait]
impl Authorizer for OAuthState {
    /// Create a code which allows retrieval of a bearer token at a later time.
    async fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        let auth_code: AuthorizationCode = grant.into();
        auth_code.save(&self.pool).await.unwrap();
        Ok(auth_code.code)
    }

    /// Retrieve the parameters associated with a token, invalidating the code
    /// in the process. In particular, a code should not be usable twice
    /// (there is no stateless implementation of an authorizer for this reason).
    async fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        match AuthorizationCode::find_code(&self.pool, code).await {
            Some(auth_code) => {
                let _result = auth_code.delete(&self.pool).await;
                Ok(Some(auth_code.into()))
            }
            None => Err(()),
        }
    }
}

#[async_trait]
impl Issuer for OAuthState {
    /// Create a token authorizing the request parameters.
    async fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        let token = OAuth2Token::from(grant);
        token.save(&self.pool).await.map_err(|_| ())?;
        Ok(token.into())
    }

    /// Refresh a token.
    async fn refresh(&mut self, refresh_token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        match OAuth2Token::find_refresh_token(&self.pool, refresh_token).await {
            Some(mut token) => {
                token
                    .refresh_and_save(&self.pool, &grant)
                    .await
                    .map_err(|_| ())?;
                Ok(token.into())
            }
            None => Err(()),
        }
    }

    /// Get the values corresponding to a bearer token.
    async fn recover_token(&mut self, access_token: &str) -> Result<Option<Grant>, ()> {
        match OAuth2Token::find_access_token(&self.pool, access_token).await {
            Some(token) => Ok(Some(token.into())),
            None => Err(()),
        }
    }

    /// Get the values corresponding to a refresh token.
    async fn recover_refresh(&mut self, refresh_token: &str) -> Result<Option<Grant>, ()> {
        match OAuth2Token::find_refresh_token(&self.pool, refresh_token).await {
            Some(token) => Ok(Some(token.into())),
            None => Err(()),
        }
    }
}

#[async_trait]
impl<'r> OwnerSolicitor<OAuthRequest<'r>> for OAuthState {
    async fn check_consent(
        &mut self,
        req: &mut OAuthRequest,
        solicitation: Solicitation<'_>,
    ) -> OwnerConsent<OAuthResponse<'r>> {
        if self.decision {
            consent_decision(self.allow, &solicitation)
        } else {
            consent_form(req, &solicitation)
        }
    }
}

#[async_trait]
impl Registrar for OAuthState {
    /// Determine the allowed scope and redirection url for the client.
    async fn bound_redirect<'a>(
        &self,
        bound: ClientUrl<'a>,
    ) -> Result<BoundClient<'a>, RegistrarError> {
        if let Some(client) = OAuth2Client::find_client_id(&self.pool, &bound.client_id).await {
            if let Ok(client_uri) = ExactUrl::new(client.redirect_uri) {
                if let Some(url) = bound.redirect_uri {
                    if url.as_ref() == &client_uri {
                        return Ok(BoundClient {
                            client_id: bound.client_id,
                            redirect_uri: Cow::Owned(RegisteredUrl::from(client_uri)),
                        });
                    }
                }
            }
        }
        Err(RegistrarError::Unspecified)
    }

    /// Finish the negotiations with the registrar.
    /// Always overrides the scope with a default scope.
    async fn negotiate<'a>(
        &self,
        bound: BoundClient<'a>,
        _scope: Option<Scope>,
    ) -> Result<PreGrant, RegistrarError> {
        match OAuth2Client::find_client_id(&self.pool, &bound.client_id).await {
            Some(client) => Ok(PreGrant {
                client_id: bound.client_id.into_owned(),
                redirect_uri: bound.redirect_uri.into_owned(),
                scope: client.scope.parse().unwrap(),
            }),
            None => Err(RegistrarError::Unspecified),
        }
    }

    /// Try to login as client with some authentication.
    /// Currently, public clients (without passphrase) are forbidden.
    async fn check(
        &self,
        client_id: &str,
        passphrase: Option<&[u8]>,
    ) -> Result<(), RegistrarError> {
        if let Some(secret) = passphrase {
            if let Some(client) = OAuth2Client::find_client_id(&self.pool, client_id).await {
                if secret == client.client_secret.as_bytes() {
                    return Ok(());
                }
            }
        }
        Err(RegistrarError::Unspecified)
    }
}

impl Extension for OAuthState {
    fn authorization(&mut self) -> Option<&mut (dyn AuthorizationExtension + Send)> {
        Some(self)
    }

    fn access_token(&mut self) -> Option<&mut (dyn AccessTokenExtension + Send)> {
        Some(self)
    }
}

#[async_trait]
impl AccessTokenExtension for OAuthState {
    async fn extend(
        &mut self,
        request: &(dyn TokenRequest + Sync),
        mut data: Extensions,
    ) -> Result<Extensions, ()> {
        let mut result_data = Extensions::new();
        let ext = Pkce::optional();
        let ext_data = data.remove(&ext);

        match AccessTokenAddon::execute(&ext, request, ext_data) {
            AddonResult::Ok => (),
            AddonResult::Data(data) => result_data.set(&ext, data),
            AddonResult::Err => return Err(()),
        }

        Ok(result_data)
    }
}

#[async_trait]
impl AuthorizationExtension for OAuthState {
    async fn extend(&mut self, request: &(dyn AuthRequest + Sync)) -> Result<Extensions, ()> {
        let mut result_data = Extensions::new();
        let ext = Pkce::optional();

        match AuthorizationAddon::execute(&ext, request) {
            AddonResult::Ok => (),
            AddonResult::Data(data) => result_data.set(&ext, data),
            AddonResult::Err => return Err(()),
        }

        Ok(result_data)
    }
}

impl<'r> Endpoint<OAuthRequest<'r>> for OAuthState {
    type Error = OAuthFailure;

    fn registrar(&self) -> Option<&(dyn Registrar + Sync)> {
        Some(self)
    }

    fn authorizer_mut(&mut self) -> Option<&mut (dyn Authorizer + Send)> {
        Some(self)
    }

    fn issuer_mut(&mut self) -> Option<&mut (dyn Issuer + Send)> {
        Some(self)
    }

    fn owner_solicitor(&mut self) -> Option<&mut (dyn OwnerSolicitor<OAuthRequest<'r>> + Send)> {
        Some(self)
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest<'r>>> {
        None
    }

    fn response(
        &mut self,
        _request: &mut OAuthRequest<'r>,
        _kind: Template,
    ) -> Result<OAuthResponse<'r>, Self::Error> {
        Ok(OAuthResponse::new())
    }

    fn error(&mut self, err: OAuthError) -> Self::Error {
        err.into()
    }

    fn web_error(&mut self, err: WebError) -> Self::Error {
        err.into()
    }

    fn extension(&mut self) -> Option<&mut (dyn Extension + Send)> {
        Some(self)
    }
}

fn consent_form<'r>(
    req: &mut OAuthRequest,
    solicitation: &Solicitation<'_>,
) -> OwnerConsent<OAuthResponse<'r>> {
    let query = req.query.as_ref().unwrap();
    let code_challenge = query
        .unique_value("code_challenge")
        .unwrap_or(Cow::Borrowed(""))
        .to_string();
    let code_challenge_method = query
        .unique_value("code_challenge_method")
        .unwrap_or(Cow::Borrowed(""))
        .to_string();

    let grant = solicitation.pre_grant();
    let state = solicitation.state();
    let scope = grant.scope.to_string();
    let mut extra = vec![
        ("response_type", "code"),
        ("client_id", grant.client_id.as_str()),
        ("redirect_uri", grant.redirect_uri.as_str()),
        ("scope", &scope),
        ("code_challenge", &code_challenge),
        ("code_challenge_method", &code_challenge_method),
    ];
    if let Some(state) = state {
        extra.push(("state", state));
    }

    let location = format!("/consent?{}", serde_urlencoded::to_string(extra).unwrap());
    OwnerConsent::InProgress(
        Response::build()
            .status(http::Status::Found)
            .header(http::Header::new("Location", location))
            .finalize()
            .into(),
    )
}

fn consent_decision<'r>(allowed: bool, _: &Solicitation) -> OwnerConsent<OAuthResponse<'r>> {
    if allowed {
        // FIXME: get rid of the dummy
        OwnerConsent::Authorized("dummy".into())
    } else {
        OwnerConsent::Denied
    }
}
