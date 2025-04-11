-- add old-type address column
ALTER TABLE wireguard_network_device
ADD COLUMN wireguard_ip_old inet NOT NULL;

-- copy the first element of new column to old column
-- all further addresses will be lost
UPDATE wireguard_network_device
SET wireguard_ip_old = wireguard_ip[1];

-- drop the "new" column
ALTER TABLE wireguard_network_device
DROP COLUMN wireguard_ip;

-- rename the column
ALTER TABLE wireguard_network_device
RENAME COLUMN wireguard_ip_old TO wireguard_ip;
