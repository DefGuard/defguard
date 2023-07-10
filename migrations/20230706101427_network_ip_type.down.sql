ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_ip type text;

ALTER TABLE wireguard_network_device ALTER COLUMN device_id DROP NOT NULL;
ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_network_id DROP NOT NULL;

DROP INDEX peer_stats_device_id_collected_at;
CREATE INDEX peer_stats_device_id_collected_at on wireguard_peer_stats (device_id, collected_at);
