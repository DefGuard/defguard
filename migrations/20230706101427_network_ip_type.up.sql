ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_ip type inet USING wireguard_ip::inet;

ALTER TABLE wireguard_network_device ALTER COLUMN device_id SET NOT NULL;
ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_network_id SET NOT NULL;

DROP INDEX peer_stats_device_id_collected_at;
CREATE INDEX peer_stats_device_id_collected_at on wireguard_peer_stats (device_id, network, collected_at DESC);
