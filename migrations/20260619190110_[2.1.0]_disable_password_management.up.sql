ALTER TABLE settings
    ADD COLUMN ldap_disable_password_management boolean NOT NULL DEFAULT false;

ALTER TABLE openidprovider
    ADD COLUMN disable_password_management boolean NOT NULL DEFAULT false;
