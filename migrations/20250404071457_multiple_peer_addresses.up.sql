-- add new address column
ALTER TABLE wireguard_network_device
ADD COLUMN wireguard_ip_new inet[] NOT NULL DEFAULT '{}';

-- copy and convert existing IPs into arrays
UPDATE wireguard_network_device
SET wireguard_ip_new = ARRAY[wireguard_ip];

-- drop the old column
ALTER TABLE wireguard_network_device
DROP COLUMN wireguard_ip;

-- rename the new column to the original name
ALTER TABLE wireguard_network_device
RENAME COLUMN wireguard_ip_new TO wireguard_ip;
