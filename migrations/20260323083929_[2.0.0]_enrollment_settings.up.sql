ALTER TABLE settings
    ADD COLUMN enrollment_send_welcome_email BOOLEAN NOT NULL DEFAULT TRUE;
