ALTER TABLE settings
    RENAME COLUMN disable_stats_purge TO enable_stats_purge;

UPDATE settings
SET enable_stats_purge = NOT enable_stats_purge;
