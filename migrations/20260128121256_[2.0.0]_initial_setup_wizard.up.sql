ALTER TABLE settings ADD COLUMN initial_setup_completed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN defguard_url TEXT NOT NULL DEFAULT 'http://localhost:8000';
ALTER TABLE settings ADD COLUMN default_admin_group_name TEXT NOT NULL DEFAULT 'admin';
ALTER TABLE settings ADD COLUMN authentication_period_days INTEGER NOT NULL DEFAULT 7;
ALTER TABLE settings ADD COLUMN mfa_code_timeout_seconds INTEGER NOT NULL DEFAULT 60;
