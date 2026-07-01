ALTER TABLE openidprovider
    DROP COLUMN disable_password_management;

ALTER TABLE settings
    DROP COLUMN ldap_disable_password_management;
