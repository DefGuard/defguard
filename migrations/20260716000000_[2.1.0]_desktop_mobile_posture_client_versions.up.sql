ALTER TABLE device_posture
    RENAME COLUMN min_client_version TO min_desktop_client_version;

ALTER TABLE device_posture
    ADD COLUMN min_mobile_client_version text;
