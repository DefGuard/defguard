CREATE TYPE dirsync_target AS ENUM (
    'all',
    'users',
    'groups'
);

ALTER TABLE openidprovider ADD COLUMN directory_sync_target dirsync_target DEFAULT 'all'::dirsync_target NOT NULL;
