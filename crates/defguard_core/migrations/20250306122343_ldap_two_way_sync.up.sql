ALTER TABLE "user"
ADD COLUMN from_ldap BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TYPE ldap_sync_status AS ENUM ('insync', 'outofsync');

ALTER TABLE settings
ADD COLUMN ldap_sync_status ldap_sync_status NOT NULL DEFAULT 'outofsync';

ALTER TABLE settings
ADD COLUMN ldap_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE settings
ADD COLUMN ldap_sync_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE settings
ADD COLUMN ldap_is_authoritative BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE settings
ADD COLUMN ldap_sync_interval int4 NOT NULL DEFAULT 300;

ALTER TABLE settings
ADD COLUMN ldap_user_auxiliary_obj_classes TEXT[] NOT NULL DEFAULT ARRAY['simpleSecurityObject', 'sambaSamAccount'];

ALTER TABLE settings ADD COLUMN ldap_uses_ad BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE "user" ADD COLUMN ldap_pass_randomized BOOLEAN NOT NULL DEFAULT FALSE;
