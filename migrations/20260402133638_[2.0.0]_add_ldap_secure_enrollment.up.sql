ALTER TABLE settings ADD COLUMN ldap_remote_enrollment_enabled bool NOT NULL DEFAULT false;
ALTER TABLE settings ADD COLUMN ldap_remote_enrollment_send_invite bool NOT NULL DEFAULT false;

ALTER TABLE "user" ADD COLUMN ldap_remote_enrollment_completed BOOLEAN NOT NULL DEFAULT false;
-- set to true for all existing LDAP users
UPDATE "user" SET ldap_remote_enrollment_completed = true WHERE from_ldap = true;
