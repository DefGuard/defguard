-- maybe switch to default true since it was required up to this point?
ALTER TABLE settings ADD COLUMN ldap_samba_enabled BOOLEAN DEFAULT FALSE;
