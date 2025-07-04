-- Add up migration script here
ALTER TABLE settings DROP COLUMN oauth_enabled;

ALTER TABLE settings DROP COLUMN web3_enabled;
