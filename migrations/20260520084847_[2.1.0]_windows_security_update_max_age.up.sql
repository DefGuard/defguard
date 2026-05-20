ALTER TABLE device_posture_os_rule
    DROP COLUMN windows_security_update_current,
    ADD COLUMN windows_security_update_max_age integer;
