-- add new address column
ALTER TABLE wireguard_network_device
ADD COLUMN wireguard_ips inet[] NOT NULL DEFAULT '{}';

-- copy and convert existing IPs into arrays
UPDATE wireguard_network_device
SET wireguard_ips = ARRAY[wireguard_ip];

-- drop the old column
ALTER TABLE wireguard_network_device
DROP COLUMN wireguard_ip;
