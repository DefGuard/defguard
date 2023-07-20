use crate::{
    auth::failed_login::FailedLoginMap,
    config::DefGuardConfig,
    db::{AppEvent, DbPool, GatewayEvent, WebHook},
    license::License,
};
use reqwest::Client;
use rocket::serde::json::serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::spawn,
};
use webauthn_rs::prelude::*;

pub struct AppState {
    pub config: DefGuardConfig,
    pub pool: DbPool,
    tx: UnboundedSender<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    pub license: License,
    pub webauthn: Webauthn,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
}

impl AppState {
    pub fn trigger_action(&self, event: AppEvent) {
        let event_name = event.name().to_owned();
        match self.tx.send(event) {
            Ok(_) => info!("Sent trigger {}", event_name),
            Err(err) => error!("Error sending trigger {}: {}", event_name, err),
        }
    }
    /// Handle webhook events
    async fn handle_triggers(pool: DbPool, mut rx: UnboundedReceiver<AppEvent>) {
        let reqwest_client = Client::builder().user_agent("reqwest").build().unwrap();
        while let Some(msg) = rx.recv().await {
            debug!("WebHook triggered");
            debug!("Retrieving webhooks");
            if let Ok(webhooks) = WebHook::all_enabled(&pool, &msg).await {
                info!("Found webhooks: {:#?}", webhooks);
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
                        .get(&webhook.url)
                        .bearer_auth(&webhook.token)
                        .header("X-DefGuard-Event", event)
                        .json(&payload)
                        .send()
                        .await
                    {
                        Ok(res) => {
                            info!("Trigger sent to {}, status {}", webhook.url, res.status());
                        }
                        Err(err) => {
                            error!("Error sending trigger to {}: {}", webhook.url, err);
                        }
                    }
                }
            }
        }
    }

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server
    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        if let Err(err) = self.wireguard_tx.send(event) {
            error!("Error sending wireguard event {}", err);
        }
    }

    /// Sends multiple events to be handled by gateway GRPC server
    pub fn send_multiple_wireguard_events(&self, events: Vec<GatewayEvent>) {
        debug!("Sending {} wireguard events", events.len());
        for event in events {
            self.send_wireguard_event(event)
        }
    }

    /// Create application state
    pub async fn new(
        config: DefGuardConfig,
        pool: DbPool,
        tx: UnboundedSender<AppEvent>,
        rx: UnboundedReceiver<AppEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        license: License,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
    ) -> Self {
        spawn(Self::handle_triggers(pool.clone(), rx));

        let webauthn_builder = WebauthnBuilder::new(
            config
                .webauthn_rp_id
                .as_ref()
                .expect("Webauth RP ID configuration is required"),
            &config.url,
        )
        .expect("Invalid WebAuthn configuration");
        let webauthn = webauthn_builder
            .build()
            .expect("Invalid WebAuthn configuration");

        Self {
            config,
            pool,
            tx,
            wireguard_tx,
            license,
            webauthn,
            failed_logins,
        }
    }
}
