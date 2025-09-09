use std::time::Duration;

use chrono::NaiveDateTime;
use defguard_version::{DefguardComponent, tracing::VersionInfo};
use semver::Version;
use serde::Serialize;
use sqlx::PgPool;
use tokio::{sync::mpsc::UnboundedSender, time::sleep};
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{Id, Settings},
    grpc::MIN_GATEWAY_VERSION,
    handlers::mail::{send_gateway_disconnected_email, send_gateway_reconnected_email},
    mail::Mail,
};

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GatewayState {
    pub uid: Uuid,
    pub connected: bool,
    pub network_id: Id,
    pub network_name: String,
    pub name: Option<String>,
    pub hostname: String,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub mail_tx: UnboundedSender<Mail>,
    #[serde(skip)]
    pub pending_notification_cancel_token: Option<CancellationToken>,
    #[schema(value_type = String)]
    pub version: Version,
}

impl GatewayState {
    #[must_use]
    pub fn new<S: Into<String>>(
        network_id: Id,
        network_name: S,
        hostname: S,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
        version: Version,
    ) -> Self {
        Self {
            uid: Uuid::new_v4(),
            connected: false,
            network_id,
            network_name: network_name.into(),
            name,
            hostname: hostname.into(),
            connected_at: None,
            disconnected_at: None,
            mail_tx,
            pending_notification_cancel_token: None,
            version,
        }
    }

    /// Checks if gateway disconnect notification should be sent.
    pub(super) fn handle_disconnect_notification(&mut self, pool: &PgPool) {
        debug!("Checking if gateway disconnect notification needs to be sent");
        let settings = Settings::get_current_settings();
        if settings.gateway_disconnect_notifications_enabled {
            let delay = Duration::from_secs(
                60 * settings.gateway_disconnect_notifications_inactivity_threshold as u64,
            );
            self.send_disconnect_notification(pool, delay);
        }
    }

    /// Send gateway disconnected notification
    /// Sends notification only if last notification time is bigger than specified in config
    fn send_disconnect_notification(&mut self, pool: &PgPool, delay: Duration) {
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();

        debug!(
            "Scheduling gateway disconnect email notification for {hostname} to be sent in \
            {delay:?}"
        );
        // use cancellation token to abort sending if gateway reconnects during the delay
        // we should never need to cancel a previous token since that would've been done on reconnect
        assert!(self.pending_notification_cancel_token.is_none());
        let cancellation_token = CancellationToken::new();
        self.pending_notification_cancel_token = Some(cancellation_token.clone());

        // notification is not supposed to be sent immediately, so we instead schedule a
        // background task with a configured delay
        tokio::spawn(async move {
            tokio::select! {
                () = async {
                    sleep(delay).await;
                    debug!("Gateway disconnect notification delay has passed. \
                        Trying to send email...");
                    if let Err(e) = send_gateway_disconnected_email(name, network_name, &hostname,
                        &mail_tx, &pool)
                    .await
                    {
                        error!("Failed to send gateway disconnect notification: {e}");
                    } else {
                        info!("Gateway {hostname} disconnected. Email notification sent",);
                    }
                } => {
                    debug!("Scheduled gateway disconnect notification for {hostname} has been \
                        sent");
                },
                () = cancellation_token.cancelled() => {
                    info!("Scheduled gateway disconnect notification for {hostname} cancelled");
                }
            }
        });
    }

    /// Checks if gateway disconnect notification should be sent.
    pub(super) fn handle_reconnect_notification(&mut self, pool: &PgPool) {
        debug!("Checking if gateway reconnect notification needs to be sent");
        let settings = Settings::get_current_settings();
        if settings.gateway_disconnect_notifications_reconnect_notification_enabled {
            self.send_reconnect_notification(pool);
        }
    }

    /// Send gateway disconnected notification
    /// Sends notification only if last notification time is bigger than specified in config
    fn send_reconnect_notification(&mut self, pool: &PgPool) {
        debug!("Sending gateway reconnect email notification");
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();
        tokio::spawn(async move {
            if let Err(e) =
                send_gateway_reconnected_email(name, network_name, &hostname, &mail_tx, &pool).await
            {
                error!("Failed to send gateway reconnect notification: {e}");
            } else {
                info!("Gateway {hostname} reconnected. Email notification sent",);
            }
        });
    }

    /// Cancels disconnect notification if one is scheduled to be sent
    pub(super) fn cancel_pending_disconnect_notification(&mut self) {
        debug!(
            "Checking if there's a gateway disconnect notification for {} pending which needs \
            to be cancelled",
            self.hostname
        );
        if let Some(token) = &self.pending_notification_cancel_token {
            debug!(
                "Cancelling pending gateway disconnect notification for {}",
                self.hostname
            );
            token.cancel();
            self.pending_notification_cancel_token = None;
        }
    }

	#[allow(dead_code)]
    pub(super) fn as_version_info(&self) -> VersionInfo {
        VersionInfo {
            component: Some(DefguardComponent::Gateway),
            info: None,
            version: Some(self.version.to_string()),
            is_supported: self.version >= MIN_GATEWAY_VERSION,
        }
    }
}
