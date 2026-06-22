ALTER TYPE activity_log_module ADD VALUE 'ldap_sync';

ALTER TABLE activity_log_event
    ALTER COLUMN user_id DROP NOT NULL;
