use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use sqlx::PgPool;
use tokio::{
    sync::mpsc::UnboundedReceiver,
    time::{Duration, interval},
};
use tracing::{debug, info};

const MESSAGE_LIMIT: usize = 100;
const SESSION_UPDATE_INTERVAL: u64 = 60;

pub async fn run_session_manager(
    pool: PgPool,
    mut peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
) {
    info!("Starting VPN client session manager service");
    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));

    loop {
        // receive next batch of peer stats messages
        // if no message is received within `SESSION_UPDATE_INTERVAL` trigger session status refresh anyway
        let mut message_buffer: Vec<PeerStatsUpdate> = Vec::with_capacity(MESSAGE_LIMIT);
        let message_count = tokio::select! {
            message_count = peer_stats_rx.recv_many(&mut message_buffer, MESSAGE_LIMIT) => message_count,
            _ = session_update_timer.tick() => {
                debug!("No wireguard peer stats updates received in last {SESSION_UPDATE_INTERVAL}. Triggering session status update.");
                continue;
            }

        };

        debug!("Processing batch of {message_count} peer stats updates");
    }
}
