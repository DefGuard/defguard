CREATE TYPE initial_setup_step AS ENUM (
    'welcome',
    'admin_user',
    'general_configuration',
    'ca',
    'ca_summary',
    'edge_component',
    'confirmation',
    'finished'
);

ALTER TABLE settings
ADD COLUMN initial_setup_step initial_setup_step NOT NULL DEFAULT 'welcome',
ADD COLUMN default_admin_id BIGINT NULL;

ALTER TABLE settings
ADD CONSTRAINT fk_default_admin
FOREIGN KEY (default_admin_id) REFERENCES "user"(id)
ON DELETE SET NULL;
