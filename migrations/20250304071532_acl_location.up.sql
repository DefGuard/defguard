ALTER TABLE wireguard_network ADD COLUMN acl_enabled boolean NOT NULL default false;
ALTER TABLE wireguard_network ADD COLUMN acl_default_allow boolean NOT NULL default false;
ALTER TABLE aclrule ADD COLUMN enabled boolean NOT NULL default true;
