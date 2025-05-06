use std::sync::{Arc, Mutex};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use reqwest::Client;
use secrecy::ExposeSecret;
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::spawn,
};
use webauthn_rs::prelude::*;

use crate::{
    auth::failed_login::FailedLoginMap,
    db::{AppEvent, GatewayEvent, WebHook},
    error::WebError,
    event_router::events::MainEvent,
    grpc::gateway::{send_multiple_wireguard_events, send_wireguard_event},
    mail::Mail,
    server_config,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    tx: UnboundedSender<AppEvent>,
    pub wireguard_tx: Sender<GatewayEvent>,
    pub mail_tx: UnboundedSender<Mail>,
    pub webauthn: Arc<Webauthn>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    key: Key,
    pub event_tx: UnboundedSender<MainEvent>,
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
    async fn handle_triggers(pool: PgPool, mut rx: UnboundedReceiver<AppEvent>) {
        let reqwest_client = Client::builder().user_agent("reqwest").build().unwrap();
        while let Some(msg) = rx.recv().await {
            debug!("WebHook triggered");
            debug!("Retrieving webhooks");
            if let Ok(webhooks) = WebHook::all_enabled(&pool, &msg).await {
                debug!("Found webhooks: {webhooks:?}");
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

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server.
    /// Convenience wrapper around [`send_wireguard_event`]
    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        send_wireguard_event(event, &self.wireguard_tx);
    }

    /// Sends multiple events to be handled by gateway GRPC server.
    /// Convenience wrapper around [`send_multiple_wireguard_events`]
    pub fn send_multiple_wireguard_events(&self, events: Vec<GatewayEvent>) {
        send_multiple_wireguard_events(events, &self.wireguard_tx);
    }

    /// Sends event to the main event router
    ///
    /// This method is fallible since events are used for communication between services
    pub fn send_event(&self, event: MainEvent) -> Result<(), WebError> {
        Ok(self.event_tx.send(event)?)
    }

    /// Create application state
    pub fn new(
        pool: PgPool,
        tx: UnboundedSender<AppEvent>,
        rx: UnboundedReceiver<AppEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
        event_tx: UnboundedSender<MainEvent>,
    ) -> Self {
        spawn(Self::handle_triggers(pool.clone(), rx));

        let config = server_config();
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
            pool,
            tx,
            wireguard_tx,
            mail_tx,
            webauthn,
            failed_logins,
            key,
            event_tx,
        }
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}
