use std::sync::{Arc, Mutex, RwLock};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use defguard_common::{
    config::server_config, db::models::Settings, types::proxy::ProxyControlMessage,
};
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
    db::{AppEvent, WebHook},
    error::WebError,
    events::ApiEvent,
    grpc::gateway::{events::GatewayEvent, send_multiple_wireguard_events, send_wireguard_event},
    version::IncompatibleComponents,
};

const X_DEFGUARD_EVENT: &str = "x-defguard-event";

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    tx: UnboundedSender<AppEvent>,
    pub wireguard_tx: Sender<GatewayEvent>,
    pub webauthn: Arc<Webauthn>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    key: Key,
    pub event_tx: UnboundedSender<ApiEvent>,
    pub incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    pub proxy_control_tx: tokio::sync::mpsc::Sender<ProxyControlMessage>,
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
                        (json!({"username": username}), "user_deleted")
                    }
                    AppEvent::HWKeyProvision(data) => (json!(data), "user_keys"),
                };
                for webhook in webhooks {
                    match reqwest_client
                        .post(&webhook.url)
                        .bearer_auth(&webhook.token)
                        .header(X_DEFGUARD_EVENT, event)
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
    pub fn emit_event(&self, event: ApiEvent) -> Result<(), WebError> {
        Ok(self.event_tx.send(event)?)
    }

    /// Create application state
    pub fn new(
        pool: PgPool,
        tx: UnboundedSender<AppEvent>,
        rx: UnboundedReceiver<AppEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
        event_tx: UnboundedSender<ApiEvent>,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        proxy_control_tx: tokio::sync::mpsc::Sender<ProxyControlMessage>,
    ) -> Self {
        spawn(Self::handle_triggers(pool.clone(), rx));

        let config = server_config();
        let url = Settings::url().expect("Invalid Defguard URL configuration");
        let webauthn_builder = WebauthnBuilder::new(
            config
                .webauthn_rp_id
                .as_ref()
                .expect("Webauth RP ID configuration is required"),
            &url,
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
            webauthn,
            failed_logins,
            key,
            event_tx,
            incompatible_components,
            proxy_control_tx,
        }
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}
