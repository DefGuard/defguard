ALTER TABLE openidprovider ADD COLUMN directory_sync_group_match TEXT[] DEFAULT '{}' NOT NULL;
