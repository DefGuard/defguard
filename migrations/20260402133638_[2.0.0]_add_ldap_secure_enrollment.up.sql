ALTER TABLE settings ADD COLUMN ldap_remote_enrollment_enabled bool NOT NULL DEFAULT false;
ALTER TABLE settings ADD COLUMN ldap_remote_enrollment_send_invite bool NOT NULL DEFAULT false;
