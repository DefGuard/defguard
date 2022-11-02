use crate::{
    config::DefGuardConfig,
    db::{AppEvent, DbPool, GatewayEvent, WebHook},
    license::License,
};
use reqwest::Client;
use rocket::serde::json::serde_json::json;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::spawn,
};
use webauthn_rs::prelude::*;

pub struct AppState {
    pub config: DefGuardConfig,
    pub pool: DbPool,
    tx: UnboundedSender<AppEvent>,
    wireguard_tx: UnboundedSender<GatewayEvent>,
    pub license: License,
    pub webauthn: Webauthn,
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
            if let Ok(webhooks) = WebHook::all_enabled(&pool, &msg).await {
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

    /// Create application state
    pub async fn new(
        config: DefGuardConfig,
        pool: DbPool,
        tx: UnboundedSender<AppEvent>,
        rx: UnboundedReceiver<AppEvent>,
        wireguard_tx: UnboundedSender<GatewayEvent>,
        license: License,
    ) -> Self {
        spawn(Self::handle_triggers(pool.clone(), rx));

        let rp_origin = Url::parse(&config.url).expect("Invalid relying party origin");
        let webauthn_builder = WebauthnBuilder::new(&config.webauthn_rp_id, &rp_origin)
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
        }
    }
}
