ALTER TABLE settings ADD ldap_use_starttls BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD ldap_tls_cert TEXT;
