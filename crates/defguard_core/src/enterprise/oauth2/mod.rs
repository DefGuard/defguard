pub mod microsoft;

use defguard_common::db::models::settings::smtp::SmtpSettings;
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, OAuth2TokenResponse, RefreshToken,
    core::{CoreClient, CoreProviderMetadata},
    reqwest::{ClientBuilder, redirect::Policy},
};
use tracing::{debug, error};

use self::microsoft::MicrosoftOAuth2;
use super::is_business_license_active;

const OUTLOOK_DEFAULT_SCOPE: &str = "https://outlook.office365.com/.default";

#[derive(Debug, thiserror::Error)]
pub enum OAuth2Error {
    #[error("OAuth2 not configured")]
    NotConfigured,
    #[error(transparent)]
    Configuration(#[from] openidconnect::ConfigurationError),
    #[error("Open ID discovery")]
    OpenIDDiscovery,
    #[error("Refresh token exchange")]
    RefreshTokenExchange,
    #[error(transparent)]
    Reqwest(#[from] openidconnect::reqwest::Error),
    #[error(transparent)]
    Url(#[from] openidconnect::url::ParseError),
}

/// Obtain access token from Google.
async fn google_access_token(smtp_settings: &mut SmtpSettings) -> Result<String, OAuth2Error> {
    let (Some(issuer_url), Some(client_id), Some(client_secret), Some(refresh_token)) = (
        &smtp_settings.oauth_issuer_url,
        &smtp_settings.oauth_client_id,
        &smtp_settings.oauth_client_secret,
        &smtp_settings.oauth_refresh_token,
    ) else {
        error!("Google SMTP XOAUTH2 requires: issuer URL, client ID, client secret, refresh token");
        return Err(OAuth2Error::NotConfigured);
    };
    let issuer_url = IssuerUrl::new(issuer_url.into())?;
    let client_id = ClientId::new(client_id.into());
    let client_secret = ClientSecret::new(client_secret.expose_secret().into());
    let refresh_token = RefreshToken::new(refresh_token.into());

    let http_client = ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(Policy::none())
        .build()?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .map_err(|err| {
            error!("Failed OpenID Connect Discovery: {err}");
            OAuth2Error::OpenIDDiscovery
        })?;

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret));

    let token_response = client
        .exchange_refresh_token(&refresh_token)?
        .request_async(&http_client)
        .await
        .map_err(|err| {
            error!("Failed to fetch token: {err}");
            OAuth2Error::RefreshTokenExchange
        })?;

    let access_token = token_response.access_token().secret();
    debug!("Got access token");
    if let Some(expires_in) = token_response.expires_in() {
        debug!("Access token expires in {expires_in:?}");
    }
    if let Some(refresh_token) = token_response.refresh_token() {
        debug!("Got refresh token");
        // TODO: use `self.set_oauth_refresh_token`
        smtp_settings.oauth_refresh_token = Some(refresh_token.secret().into());
    }
    Ok(access_token.clone())
}

/// Obtain access token from Microsoft.
async fn microsoft_access_token(smtp_settings: &mut SmtpSettings) -> Result<String, OAuth2Error> {
    let (Some(client_id), Some(client_secret), Some(tenant_id)) = (
        &smtp_settings.oauth_client_id,
        &smtp_settings.oauth_client_secret,
        &smtp_settings.oauth_tenant_id,
    ) else {
        error!("Microsoft SMTP XOAUTH2 requires: tenant ID, client ID, client secret");
        return Err(OAuth2Error::NotConfigured);
    };

    let oauth2 = MicrosoftOAuth2::new(
        client_id.clone(),
        client_secret.expose_secret().to_string(),
        tenant_id.clone(),
        OUTLOOK_DEFAULT_SCOPE.into(),
    );
    let token = oauth2.fetch_access_token().await?;

    Ok(token.access_token())
}

/// Obtain access token for XOAUTH2 authentication.
pub async fn xoauth2_access_token(smtp_settings: &mut SmtpSettings) -> Result<String, OAuth2Error> {
    if !is_business_license_active() {
        error!("SMTP XOAUTH2 requires business license");
    } else if let Some(issuer_url) = &smtp_settings.oauth_issuer_url {
        // FIXME: baked URLs
        if issuer_url == "https://login.microsoftonline.com/common" {
            return microsoft_access_token(smtp_settings).await;
        }
        if issuer_url == "https://accounts.google.com" {
            return google_access_token(smtp_settings).await;
        }
    } else {
        error!("SMTP XOAUTH2 requires: issuer URL");
    }

    Err(OAuth2Error::NotConfigured)
}
