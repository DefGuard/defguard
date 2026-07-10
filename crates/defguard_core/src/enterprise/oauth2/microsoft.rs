use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::super::REQUEST_TIMEOUT;

const TOKEN_URL: &str = "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token";
const GRANT_TYPE: &str = "client_credentials";

#[derive(Deserialize)]
pub struct TokenResponse {
    // token_type: String,
    access_token: String,
    expires_in: u64,
}

impl TokenResponse {
    #[must_use]
    pub fn access_token(&self) -> String {
        self.access_token.clone()
    }

    #[must_use]
    pub fn expires_in(&self) -> SystemTime {
        SystemTime::now() + Duration::from_secs(self.expires_in)
    }
}

pub struct MicrosoftOAuth2 {
    client_id: String,
    client_secret: String,
    tenant_id: String,
    scope: String,
}

impl MicrosoftOAuth2 {
    #[must_use]
    pub fn new(client_id: String, client_secret: String, tenant_id: String, scope: String) -> Self {
        Self {
            client_id,
            client_secret,
            tenant_id,
            scope,
        }
    }

    pub async fn fetch_access_token(&self) -> Result<TokenResponse, reqwest::Error> {
        debug!("Querying Microsoft for OAuth2 access token.");
        let token_url = TOKEN_URL.replace("{tenant_id}", &self.tenant_id);
        let client = reqwest::Client::new();
        let response = client
            .post(&token_url)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("scope", self.scope.as_str()),
                ("grant_type", GRANT_TYPE),
            ])
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        let token_response = response.json::<TokenResponse>().await?;
        debug!("Fetched Microsoft OAuth2 access token");

        Ok(token_response)
    }
}
