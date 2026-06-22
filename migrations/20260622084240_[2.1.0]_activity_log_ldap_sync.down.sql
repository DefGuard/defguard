UPDATE activity_log_event
SET module = 'defguard'::activity_log_module
WHERE module = 'ldap_sync';

CREATE TYPE activity_log_module_new AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment',
    'posture'
);

ALTER TABLE activity_log_event
    ALTER COLUMN module TYPE activity_log_module_new USING module::TEXT::activity_log_module_new;

DROP TYPE activity_log_module;

ALTER TYPE activity_log_module_new RENAME TO activity_log_module;

UPDATE activity_log_event
SET user_id = 0
WHERE user_id IS NULL;

ALTER TABLE activity_log_event
    ALTER COLUMN user_id SET NOT NULL;
