CREATE TYPE openid_username_handling AS ENUM (
    'remove_forbidden',
    'replace_forbidden',
    'prune_email_domain'
);
ALTER TABLE settings ADD COLUMN openid_username_handling openid_username_handling NOT NULL DEFAULT 'remove_forbidden';
