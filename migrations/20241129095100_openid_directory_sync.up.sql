CREATE TYPE dirsync_user_behavior AS ENUM (
    'keep',
    'disable',
    'delete'
);

ALTER TABLE openidprovider ADD COLUMN google_service_account_key TEXT DEFAULT NULL;
ALTER TABLE openidprovider ADD COLUMN google_service_account_email TEXT DEFAULT NULL;
ALTER TABLE openidprovider ADD COLUMN admin_email TEXT DEFAULT NULL;
ALTER TABLE openidprovider ADD COLUMN directory_sync_enabled BOOLEAN DEFAULT FALSE NOT NULL;
ALTER TABLE openidprovider ADD COLUMN directory_sync_interval int4 DEFAULT 600 NOT NULL;
ALTER TABLE openidprovider ADD COLUMN directory_sync_user_behavior dirsync_user_behavior DEFAULT 'keep'::dirsync_user_behavior NOT NULL;
ALTER TABLE openidprovider ADD COLUMN directory_sync_admin_behavior dirsync_user_behavior DEFAULT 'keep'::dirsync_user_behavior NOT NULL;
