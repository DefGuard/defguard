ALTER TABLE openidprovider DROP COLUMN google_service_account_key;
ALTER TABLE openidprovider DROP COLUMN google_service_account_email;
ALTER TABLE openidprovider DROP COLUMN admin_email;
ALTER TABLE openidprovider DROP COLUMN directory_sync_enabled;
ALTER TABLE openidprovider DROP COLUMN directory_sync_interval;
ALTER TABLE openidprovider DROP COLUMN directory_sync_user_behavior;
ALTER TABLE openidprovider DROP COLUMN directory_sync_admin_behavior;
DROP TYPE dirsync_user_behavior;
