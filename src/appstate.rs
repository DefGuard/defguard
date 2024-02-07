use std::sync::{Arc, Mutex};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use openidconnect::{core::CoreRsaPrivateSigningKey, JsonWebKeyId};
use reqwest::Client;
use rsa::{
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey},
    pkcs8::{DecodePrivateKey, LineEnding},
    traits::PublicKeyParts,
    RsaPrivateKey,
};
use secrecy::ExposeSecret;
use serde_json::json;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::spawn,
};
use uaparser::UserAgentParser;
use webauthn_rs::prelude::*;

use crate::{
    auth::failed_login::FailedLoginMap,
    config::DefGuardConfig,
    db::{AppEvent, DbPool, GatewayEvent, WebHook},
    mail::Mail,
};

#[derive(Clone)]
pub struct AppState {
    pub config: DefGuardConfig,
    pub pool: DbPool,
    tx: UnboundedSender<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    pub mail_tx: UnboundedSender<Mail>,
    pub webauthn: Arc<Webauthn>,
    pub user_agent_parser: Arc<UserAgentParser>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    key: Key,
    pub rsa_key: Option<RsaPrivateKey>,
}

impl AppState {
    pub(crate) fn trigger_action(&self, event: AppEvent) {
        let event_name = event.name().to_owned();
        match self.tx.send(event) {
            Ok(()) => info!("Sent trigger {event_name}"),
            Err(err) => error!("Error sending trigger {event_name}: {err}"),
        }
    }

    /// Handle webhook events
    async fn handle_triggers(pool: DbPool, mut rx: UnboundedReceiver<AppEvent>) {
        let reqwest_client = Client::builder().user_agent("reqwest").build().unwrap();
        while let Some(msg) = rx.recv().await {
            debug!("WebHook triggered");
            debug!("Retrieving webhooks");
            if let Ok(webhooks) = WebHook::all_enabled(&pool, &msg).await {
                info!("Found webhooks: {webhooks:#?}");
                let (payload, event) = match msg {
                    AppEvent::UserCreated(user) => (json!(user), "user_created"),
                    AppEvent::UserModified(user) => (json!(user), "user_modified"),
                    AppEvent::UserDeleted(username) => {
                        (json!({ "username": username }), "user_deleted")
                    }
                    AppEvent::HWKeyProvision(data) => (json!(data), "user_keys"),
                };
                for webhook in webhooks {
                    match reqwest_client
                        .post(&webhook.url)
                        .bearer_auth(&webhook.token)
                        .header("x-defguard-event", event)
                        .json(&payload)
                        .send()
                        .await
                    {
                        Ok(res) => {
                            info!("Trigger sent to {}, status {}", webhook.url, res.status());
                        }
                        Err(err) => {
                            error!("Error sending trigger to {}: {err}", webhook.url);
                        }
                    }
                }
            }
        }
    }

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server
    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        if let Err(err) = self.wireguard_tx.send(event) {
            error!("Error sending WireGuard event {err}");
        }
    }

    /// Sends multiple events to be handled by gateway GRPC server
    pub fn send_multiple_wireguard_events(&self, events: Vec<GatewayEvent>) {
        debug!("Sending {} wireguard events", events.len());
        for event in events {
            self.send_wireguard_event(event);
        }
    }

    /// Create application state
    pub fn new(
        config: DefGuardConfig,
        pool: DbPool,
        tx: UnboundedSender<AppEvent>,
        rx: UnboundedReceiver<AppEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        user_agent_parser: Arc<UserAgentParser>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
    ) -> Self {
        // spawn webhook handler task
        spawn(Self::handle_triggers(pool.clone(), rx));

        // read RSA key if configured
        let rsa_key = match config.openid_signing_key.as_ref() {
            Some(path) => {
                info!("Using RSA OpenID signing key");
                let key = if let Ok(key) = RsaPrivateKey::read_pkcs1_pem_file(path) {
                    Ok(key)
                } else {
                    RsaPrivateKey::read_pkcs8_pem_file(path)
                }
                .expect("Failed to read RSA key from file");

                Some(key)
            }
            None => {
                info!("Using HMAC OpenID signing key");
                None
            }
        };

        let webauthn_builder = WebauthnBuilder::new(
            config
                .webauthn_rp_id
                .as_ref()
                .expect("Webauth RP ID configuration is required"),
            &config.url,
        )
        .expect("Invalid WebAuthn configuration");
        let webauthn = Arc::new(
            webauthn_builder
                .build()
                .expect("Invalid WebAuthn configuration"),
        );

        let key = Key::from(config.secret_key.expose_secret().as_bytes());

        Self {
            config,
            pool,
            tx,
            wireguard_tx,
            mail_tx,
            webauthn,
            user_agent_parser,
            failed_logins,
            key,
            rsa_key,
        }
    }

    #[must_use]
    pub fn openid_key(&self) -> Option<CoreRsaPrivateSigningKey> {
        let key = self.rsa_key.as_ref()?;
        if let Ok(pem) = key.to_pkcs1_pem(LineEnding::default()) {
            let key_id = JsonWebKeyId::new(key.n().to_str_radix(36));
            CoreRsaPrivateSigningKey::from_pem(pem.as_ref(), Some(key_id)).ok()
        } else {
            None
        }
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}
