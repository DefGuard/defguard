-- maybe switch to default true since it was required up to this point?
ALTER TABLE settings
ADD COLUMN ldap_samba_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE "user"
ADD COLUMN ldap_linked BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TYPE ldap_sync_status AS ENUM ('synced', 'desynced');

ALTER TABLE settings
ADD COLUMN ldap_sync_status ldap_sync_status NOT NULL DEFAULT 'desynced';

ALTER TABLE settings
ADD COLUMN ldap_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE settings
ADD COLUMN ldap_sync_enabled BOOLEAN NOT NULL DEFAULT FALSE;
