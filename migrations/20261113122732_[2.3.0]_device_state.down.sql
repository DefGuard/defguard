ALTER TABLE device DROP COLUMN verification;

DROP TYPE device_verification;

ALTER TABLE device DROP COLUMN state;

DROP TYPE device_state;
