ALTER TABLE device ADD COLUMN preshared_key text NULL;

-- remove previous column
ALTER TABLE wireguard_network_device DROP COLUMN preshared_key;
