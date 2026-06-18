ALTER TABLE enterprisesettings ADD COLUMN hide_download_step BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE enterprisesettings ADD COLUMN password_reset_disabled BOOLEAN NOT NULL DEFAULT false;
