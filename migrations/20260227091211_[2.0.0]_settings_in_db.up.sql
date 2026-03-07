ALTER TABLE settings
    ADD COLUMN secret_key text,
    ADD COLUMN openid_signing_key text,
    ADD COLUMN webauthn_rp_id text,
    ADD COLUMN disable_stats_purge boolean NOT NULL DEFAULT false,
    ADD COLUMN stats_purge_frequency_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN stats_purge_threshold_days int4 NOT NULL DEFAULT 30,
    ADD COLUMN enrollment_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN password_reset_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN enrollment_session_timeout_minutes int4 NOT NULL DEFAULT 10,
    ADD COLUMN password_reset_session_timeout_minutes int4 NOT NULL DEFAULT 10;
