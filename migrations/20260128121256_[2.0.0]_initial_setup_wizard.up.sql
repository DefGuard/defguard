ALTER TABLE settings
ADD COLUMN initial_setup_completed BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN defguard_url TEXT NOT NULL DEFAULT 'http://localhost:8000',
ADD COLUMN default_admin_group_name TEXT NOT NULL DEFAULT 'admin',
ADD COLUMN authentication_period_days INTEGER NOT NULL DEFAULT 7,
ADD COLUMN mfa_code_timeout_seconds INTEGER NOT NULL DEFAULT 60;
