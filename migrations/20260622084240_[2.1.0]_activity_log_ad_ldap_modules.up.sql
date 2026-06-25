ALTER TYPE activity_log_module ADD VALUE 'active_directory';
ALTER TYPE activity_log_module ADD VALUE 'ldap';

ALTER TABLE activity_log_event
    ALTER COLUMN user_id DROP NOT NULL;
