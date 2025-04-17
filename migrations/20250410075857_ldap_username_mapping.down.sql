ALTER TABLE settings DROP COLUMN ldap_user_rdn_attr;
ALTER TABLE "user" DROP COLUMN ldap_rdn;
DROP INDEX IF EXISTS unique_ldap_rdn;
