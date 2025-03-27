ALTER TABLE "user"
DROP COLUMN from_ldap;

ALTER TABLE settings
DROP COLUMN ldap_sync_status;

ALTER TABLE settings
DROP COLUMN ldap_enabled;

ALTER TABLE settings
DROP COLUMN ldap_sync_enabled;

DROP TYPE ldap_sync_status;

ALTER TABLE settings
DROP COLUMN ldap_is_authoritative;

ALTER TABLE settings
DROP COLUMN ldap_sync_interval;

ALTER TABLE settings
DROP COLUMN ldap_user_auxiliary_obj_classes;

ALTER TABLE settings
DROP COLUMN ldap_uses_ad;

ALTER TABLE "user"
DROP COLUMN ldap_pass_randomized;
