-- Restore legacy stats table
CREATE TABLE wireguard_peer_stats (
    id bigserial PRIMARY KEY,
    device_id bigint NOT NULL,
    collected_at timestamp without time zone NOT NULL DEFAULT current_timestamp,
    network bigint NOT NULL,
    endpoint text,
    upload bigint NOT NULL,
    download bigint NOT NULL,
    latest_handshake timestamp without time zone NOT NULL,
    allowed_ips text,
    FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE CASCADE
);
CREATE INDEX peer_stats_device_id_collected_at on wireguard_peer_stats (device_id, network, collected_at DESC, latest_handshake DESC NULLS LAST);


-- Restore stats view
CREATE OR REPLACE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload, 1, upload) OVER (PARTITION BY device_id, network ORDER BY collected_at), 0) upload,
        greatest(download - lag(download, 1, download) OVER (PARTITION BY device_id, network ORDER BY collected_at), 0) download,
        latest_handshake - (lag(latest_handshake, 1, latest_handshake) OVER (PARTITION BY device_id, network ORDER BY collected_at)) latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;
