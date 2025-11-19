use defguard_common::db::models::wireguard_peer_stats::WireguardPeerStats;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn run_session_manager(
    _pool: PgPool,
    _peer_stats_rx: UnboundedReceiver<WireguardPeerStats>,
) {
    unimplemented!()
}
