ALTER TABLE openidprovider ADD COLUMN okta_private_jwk TEXT DEFAULT NULL;
ALTER TABLE openidprovider ADD COLUMN okta_dirsync_client_id TEXT DEFAULT NULL;
