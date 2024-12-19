ALTER TABLE device
DROP COLUMN device_type;

ALTER TABLE device
DROP COLUMN description;

ALTER TABLE device
DROP COLUMN configured;

ALTER TABLE token
DROP CONSTRAINT enrollment_device_id_fkey;

ALTER TABLE token
DROP COLUMN device_id;

DROP TYPE device_type;
