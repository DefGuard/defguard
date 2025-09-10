-- Add down migration script here
ALTER TABLE settings ADD COLUMN ldap_enabled;
ALTER TABLE settings
DROP COLUMN ldap_url,
DROP COLUMN ldap_bind_username,
DROP COLUMN ldap_bind_password,
DROP COLUMN ldap_group_search_base,
DROP COLUMN ldap_user_search_base,
DROP COLUMN ldap_user_obj_class,
DROP COLUMN ldap_group_obj_class,
DROP COLUMN ldap_username_attr,
DROP COLUMN ldap_groupname_attr,
DROP COLUMN ldap_group_member_attr,
DROP COLUMN ldap_member_attr;
