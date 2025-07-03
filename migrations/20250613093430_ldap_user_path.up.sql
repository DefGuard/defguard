ALTER TABLE "user"
ADD COLUMN "ldap_user_path" TEXT;

ALTER TABLE "user"
DROP CONSTRAINT unique_ldap_rdn;

ALTER TABLE "user" ADD CONSTRAINT unique_ldap_rdn_path UNIQUE (ldap_user_path, ldap_rdn);
