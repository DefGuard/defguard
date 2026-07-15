UPDATE activity_log_event
SET module = 'defguard'::activity_log_module
WHERE module = 'oidc_directory_sync';

CREATE TYPE activity_log_module_new AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment',
    'posture',
    'active_directory',
    'ldap'
);

ALTER TABLE activity_log_event
    ALTER COLUMN module TYPE activity_log_module_new USING module::TEXT::activity_log_module_new;

DROP TYPE activity_log_module;

ALTER TYPE activity_log_module_new RENAME TO activity_log_module;
