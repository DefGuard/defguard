use crate::{
    db::DbPool,
    enterprise::db::openid::{AuthorizedApp, OpenIDClient, OpenIDClientAuth},
};
use chrono::Local;
use openidconnect::PkceCodeChallenge;
use rocket::{response::Redirect, FromForm};

#[derive(FromForm, Deserialize)]
pub struct OpenIDRequest {
    pub client_id: String,
    pub scope: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub state: String,
    pub nonce: Option<String>,
    pub allow: bool,
}

impl OpenIDRequest {
    pub fn verify_response(&self) -> Result<(), Redirect> {
        let response: Vec<&str> = self.response_type.split(' ').collect();
        if response.len() > 1 || !response.contains(&"code") {
            Err(Redirect::found(format!(
                "{}?error=unsupported_response_type",
                self.redirect_uri
            )))
        } else {
            Ok(())
        }
    }

    // verify redirect uri
    pub fn verify_redirect_uri(&self, client: &OpenIDClient) -> Result<(), Redirect> {
        if self.redirect_uri != client.redirect_uri {
            Err(Redirect::found(format!(
                "{}?error=unauthorized_client&error_description=client_redirect_uri_dont_match",
                self.redirect_uri
            )))
        } else {
            Ok(())
        }
    }

    /// Verify user allow
    pub fn verify_allow(&self) -> Result<(), Redirect> {
        if self.allow {
            Ok(())
        } else {
            Err(Redirect::found(format!(
                "{}?error=user_unauthorized",
                self.redirect_uri
            )))
        }
    }

    /// Verify if supported scopes
    pub fn verify_scope(&self) -> Result<(), Redirect> {
        if self.scope.to_lowercase().contains("openid") {
            Ok(())
        } else {
            Err(Redirect::found(format!(
                "{}?error=wrong_scope&error_description=scope_must_contain_openid",
                self.redirect_uri
            )))
        }
    }

    // Create authorization code and save it to database
    pub async fn create_code(
        &self,
        pool: &DbPool,
        username: &str,
        user_id: i64,
    ) -> Result<Redirect, Redirect> {
        match OpenIDClient::find_enabled_for_client_id(pool, &self.client_id).await {
            Ok(Some(client)) => {
                self.verify_allow()?;
                self.verify_scope()?;
                self.verify_response()?;
                self.verify_redirect_uri(&client)?;
                let (code, _) = PkceCodeChallenge::new_random_sha256_len(32);
                let mut client_auth = OpenIDClientAuth::new(
                    username.into(),
                    code.as_str().into(),
                    client.client_id.clone(),
                    self.state.clone(),
                    client.redirect_uri.clone(),
                    self.scope.clone(),
                    self.nonce.clone(),
                );

                match AuthorizedApp::find_by_user_and_client_id(pool, user_id, &client.client_id)
                    .await
                {
                    Ok(Some(_app)) => (),
                    Ok(None) => {
                        let date = Local::now().format("%d-%m-%Y %H:%M");
                        let mut app = AuthorizedApp::new(
                            user_id,
                            client.client_id.clone(),
                            client.home_url,
                            date.to_string(),
                            client.name,
                        );
                        app.save(pool).await.map_err(|_| {
                            Redirect::found(format!(
                                "{}?error=failed_to_save_app",
                                client.redirect_uri
                            ))
                        })?;
                    }
                    Err(err) => {
                        return Err(Redirect::found(format!(
                            "{}?error=internal_server_error&error_description={}",
                            self.redirect_uri, err
                        )))
                    }
                };

                client_auth.save(pool).await.map_err(|_| {
                    Redirect::found(format!(
                        "{}?error=failed_to_save_authorization_code",
                        client.redirect_uri
                    ))
                })?;
                info!("Created code for client: {}", client.client_id);
                Ok(Redirect::found(format!(
                    "{}?code={}&state={}",
                    client.redirect_uri,
                    code.as_str(),
                    self.state
                )))
            }
            Ok(None) => Ok(Redirect::found(format!(
                "{}error=unauthorized_client&error_description=client_id_not_found",
                self.redirect_uri
            ))),
            Err(err) => Err(Redirect::found(format!(
                "{}error=internal_server_error&error_description={}",
                self.redirect_uri, err
            ))),
        }
    }
}
