ALTER TABLE wireguard_network DROP COLUMN acl_enabled;
ALTER TABLE wireguard_network DROP COLUMN acl_default_allow;
ALTER TABLE aclrule DROP COLUMN enabled;
