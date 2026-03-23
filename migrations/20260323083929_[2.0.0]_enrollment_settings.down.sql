ALTER TABLE settings
    DROP COLUMN enrollment_macos_release_channel,
    DROP COLUMN enrollment_linux_release_channel,
    DROP COLUMN enrollment_windows_release_channel,
    DROP COLUMN enrollment_send_welcome_email,
    DROP COLUMN enrollment_show_welcome_message,
    DROP COLUMN enrollment_show_reset_password,
    DROP COLUMN enrollment_admin_custom_email,
    DROP COLUMN enrollment_admin_email_mode;

DROP TYPE enrollment_release_channel;
DROP TYPE enrollment_admin_email_mode;
