ALTER TABLE settings ADD ldap_user_rdn_attr TEXT;
ALTER TABLE "user" ADD ldap_rdn TEXT;
ALTER TABLE "user" ADD CONSTRAINT unique_ldap_rdn UNIQUE (ldap_rdn);
UPDATE "user" SET ldap_rdn = username WHERE ldap_rdn IS NULL;
