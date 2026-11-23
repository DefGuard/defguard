ALTER TABLE device ADD COLUMN preshared_key text NULL;

ALTER TABLE device ADD CONSTRAINT device_preshared_key_network_only
    CHECK (preshared_key IS NULL OR device_type = 'network');
