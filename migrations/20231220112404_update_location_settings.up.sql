ALTER TABLE wireguard_network ADD COLUMN mfa_enabled bool NOT NULL DEFAULT false;
ALTER TABLE wireguard_network ADD COLUMN keepalive_interval int4 NOT NULL DEFAULT 25;
ALTER TABLE wireguard_network ADD COLUMN peer_disconnect_threshold int4 NOT NULL DEFAULT 75;
