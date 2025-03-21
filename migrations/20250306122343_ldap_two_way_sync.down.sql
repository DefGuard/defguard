ALTER TABLE settings
DROP COLUMN ldap_samba_enabled;

ALTER TABLE "user"
DROP COLUMN ldap_linked;

ALTER TABLE settings
DROP COLUMN ldap_sync_status;

ALTER TABLE settings
DROP COLUMN ldap_enabled;

ALTER TABLE settings
DROP COLUMN ldap_sync_enabled;

DROP TYPE ldap_sync_status;
