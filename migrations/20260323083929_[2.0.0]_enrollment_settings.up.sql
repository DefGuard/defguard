CREATE TYPE enrollment_admin_email_mode AS ENUM (
    'initiating_admin',
    'hidden',
    'custom_email'
);

CREATE TYPE enrollment_release_channel AS ENUM (
    'stable',
    'beta',
    'alpha'
);

ALTER TABLE settings
    ADD COLUMN enrollment_admin_email_mode enrollment_admin_email_mode NOT NULL DEFAULT 'initiating_admin',
    ADD COLUMN enrollment_admin_custom_email TEXT NULL,
    ADD COLUMN enrollment_show_reset_password BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN enrollment_show_welcome_message BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN enrollment_send_welcome_email BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN enrollment_windows_release_channel enrollment_release_channel NOT NULL DEFAULT 'stable',
    ADD COLUMN enrollment_linux_release_channel enrollment_release_channel NOT NULL DEFAULT 'stable',
    ADD COLUMN enrollment_macos_release_channel enrollment_release_channel NOT NULL DEFAULT 'stable';
