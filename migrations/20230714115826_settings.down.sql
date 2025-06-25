-- Add down migration script here
ALTER TABLE settings ADD COLUMN web3_enabled boolean NOT NULL;
ALTER TABLE settings ADD COLUMN oauth_enabled boolean NOT NULL;
