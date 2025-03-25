ALTER TABLE "user"
DROP COLUMN ldap_linked;

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
DROP COLUMN ldap_user_obj_classes;
