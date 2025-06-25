-- add old-type address column
ALTER TABLE wireguard_network_device
ADD COLUMN wireguard_ip inet;

-- copy the first element of new column to old column
-- all further addresses will be lost
UPDATE wireguard_network_device
SET wireguard_ip = wireguard_ips[1];

-- add not-null modifier to old-type address column
ALTER TABLE wireguard_network_device
ALTER COLUMN wireguard_ip SET NOT NULL;

-- drop the "new" column
ALTER TABLE wireguard_network_device
DROP COLUMN wireguard_ips;
