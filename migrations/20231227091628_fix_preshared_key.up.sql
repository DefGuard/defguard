ALTER TABLE wireguard_network_device ADD COLUMN preshared_key text NULL;

-- remove previous column
ALTER TABLE device DROP COLUMN preshared_key;
