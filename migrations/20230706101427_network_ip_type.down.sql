ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_ip type text;

ALTER TABLE wireguard_network_device ALTER COLUMN device_id DROP NOT NULL;
ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_network_id DROP NOT NULL;
