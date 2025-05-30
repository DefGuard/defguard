-- Add up migration script here
CREATE TABLE wireguard_network_device (
  device_id bigint REFERENCES "device"(id) ON DELETE CASCADE,
  wireguard_network_id bigint REFERENCES "wireguard_network"(id) ON DELETE CASCADE,
  wireguard_ip text NOT NULL,
  CONSTRAINT device_network UNIQUE (device_id, wireguard_network_id)
);
-- migrate data from device
INSERT INTO wireguard_network_device(device_id, wireguard_network_id, wireguard_ip)
SELECT d.id, (SELECT id from wireguard_network ORDER BY id ASC LIMIT 1), d.wireguard_ip FROM device d;

ALTER TABLE device DROP COLUMN wireguard_ip;
