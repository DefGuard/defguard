-- add new enum type without posture module
CREATE TYPE activity_log_module_new AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment'
);

-- restore posture events to the modules used before posture was split out
UPDATE activity_log_event
SET module = CASE
    WHEN event IN ('device_posture_check_passed', 'device_posture_check_failed')
        THEN 'vpn'::activity_log_module
    ELSE 'defguard'::activity_log_module
END
WHERE module = 'posture';

-- update activity log table to use new enum
ALTER TABLE activity_log_event
    ALTER COLUMN module TYPE activity_log_module_new USING module::TEXT::activity_log_module_new;

-- remove old enum
DROP TYPE activity_log_module;

-- rename new enum
ALTER TYPE activity_log_module_new RENAME TO activity_log_module;
