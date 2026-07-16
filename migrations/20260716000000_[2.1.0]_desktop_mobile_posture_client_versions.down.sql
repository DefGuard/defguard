ALTER TABLE device_posture
    DROP COLUMN min_mobile_client_version;

ALTER TABLE device_posture
    RENAME COLUMN min_desktop_client_version TO min_client_version;
