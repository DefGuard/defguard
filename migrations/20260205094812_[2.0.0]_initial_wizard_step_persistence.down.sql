ALTER TABLE settings DROP CONSTRAINT fk_default_admin;

ALTER TABLE settings
DROP COLUMN initial_setup_step,
DROP COLUMN default_admin_id;

DROP TYPE initial_setup_step;
