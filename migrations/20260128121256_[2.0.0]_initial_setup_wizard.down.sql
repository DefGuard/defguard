ALTER TABLE settings
    DROP COLUMN initial_setup_completed,
    DROP COLUMN defguard_url,
    DROP COLUMN default_admin_group_name,
    DROP COLUMN authentication_period_days,
    DROP COLUMN mfa_code_timeout_seconds;
