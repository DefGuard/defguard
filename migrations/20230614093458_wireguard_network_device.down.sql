-- Add down migration script here
ALTER TABLE device ADD COLUMN wireguard_ip text;
DELETE FROM device;
DROP TABLE wireguard_network_device;
