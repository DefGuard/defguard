use defguard_common::db::models::settings::smtp::SmtpSettings;
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, OAuth2TokenResponse, RefreshToken,
    core::{CoreClient, CoreProviderMetadata},
    reqwest::{ClientBuilder, redirect::Policy},
};
use tracing::{debug, error};

use super::MailError;

/// Obtain access token for XOAUTH2 authentication.
pub(super) async fn obtain_access_token(
    smtp_settings: &mut SmtpSettings,
) -> Result<String, MailError> {
    let (Some(issuer_url), Some(client_id), Some(client_secret), Some(refresh_token)) = (
        &smtp_settings.oauth_issuer_url,
        &smtp_settings.oauth_client_id,
        &smtp_settings.oauth_client_secret,
        &smtp_settings.oauth_refresh_token,
    ) else {
        error!("SMTP XOAUTH requires: issuer URL, client ID, client secret, and refresh token");
        return Err(MailError::SmtpNotConfigured);
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
            MailError::OpenIDDiscovery
        })?;

    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret));

    let token_response = client
        .exchange_refresh_token(&refresh_token)?
        .request_async(&http_client)
        .await
        .map_err(|err| {
            error!("Failed to fetch token: {err}");
            MailError::RefreshTokenExchange
        })?;

    let access_token = token_response.access_token().secret();
    debug!("Got access token");
    if let Some(expires_in) = token_response.expires_in() {
        debug!("Access token expires in:\n{expires_in:?}\n");
    }
    if let Some(refresh_token) = token_response.refresh_token() {
        debug!("Got refresh token");
        // TODO: use `smtp_settings.set_oauth_refresh_token`
        smtp_settings.oauth_refresh_token = Some(refresh_token.secret().into());
    }
    Ok(access_token.clone())
}
