-- Add up migration script here
ALTER TABLE settings
ADD COLUMN ldap_url text,
ADD COLUMN ldap_bind_username text DEFAULT 'cn=admin,dc=example,dc=org',
ADD COLUMN ldap_bind_password text,
ADD COLUMN ldap_group_member_attr text DEFAULT 'uniqueMember',
ADD COLUMN ldap_group_search_base text DEFAULT 'ou=groups,dc=example,dc=org',
ADD COLUMN ldap_groupname_attr text DEFAULT 'cn',
ADD COLUMN ldap_user_search_base text DEFAULT 'ou=users,dc=example,dc=org',
ADD COLUMN ldap_user_obj_class text DEFAULT 'inetOrgPerson',
ADD COLUMN ldap_group_obj_class text DEFAULT 'groupOfUniqueNames',
ADD COLUMN ldap_username_attr text DEFAULT 'cn',
ADD COLUMN ldap_member_attr text DEFAULT 'memberOf';

ALTER TABLE settings
DROP COLUMN ldap_enabled;
