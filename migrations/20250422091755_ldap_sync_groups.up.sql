ALTER TABLE settings ADD COLUMN ldap_sync_groups TEXT[] NOT NULL DEFAULT '{}';
