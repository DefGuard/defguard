CREATE OR REPLACE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload) OVER (PARTITION BY device_id ORDER BY collected_at), 0) upload,
        greatest(download - lag(download) OVER (PARTITION BY device_id ORDER BY collected_at), 0) download,
        latest_handshake - (lag(latest_handshake) OVER (PARTITION BY device_id ORDER BY collected_at)) latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;
