ALTER TABLE settings
    RENAME COLUMN disable_stats_purge TO enable_stats_purge;

ALTER TABLE settings
    ALTER COLUMN enable_stats_purge SET DEFAULT true;

ALTER TABLE settings
    DROP COLUMN webauthn_rp_id;

UPDATE settings
    SET enable_stats_purge = NOT enable_stats_purge;
