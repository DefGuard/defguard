ALTER TABLE settings
    ADD COLUMN webauthn_rp_id text;

UPDATE settings
    SET enable_stats_purge = NOT enable_stats_purge;

ALTER TABLE settings
    ALTER COLUMN enable_stats_purge SET DEFAULT false;

ALTER TABLE settings
    RENAME COLUMN enable_stats_purge TO disable_stats_purge;
