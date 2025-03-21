ALTER TABLE aclrule ADD COLUMN allow_all_network_devices boolean NOT NULL DEFAULT false;
ALTER TABLE aclrule ADD COLUMN deny_all_network_devices boolean NOT NULL DEFAULT false;
