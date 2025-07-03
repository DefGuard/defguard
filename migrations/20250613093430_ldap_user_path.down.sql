ALTER TABLE "user"
DROP CONSTRAINT unique_ldap_rdn_path;

ALTER TABLE "user"
DROP COLUMN "ldap_user_path";

ALTER TABLE "user" ADD CONSTRAINT unique_ldap_rdn UNIQUE (ldap_rdn);
