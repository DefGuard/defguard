CREATE TYPE device_type AS ENUM (
    'user',
    'network'
);

ALTER TABLE device ADD COLUMN device_type device_type DEFAULT 'user'::device_type NOT NULL;
ALTER TABLE device ADD COLUMN description TEXT;
ALTER TABLE device ADD COLUMN configured BOOLEAN DEFAULT TRUE NOT NULL;
ALTER TABLE token ADD COLUMN device_id bigint;
ALTER TABLE token ADD CONSTRAINT enrollment_device_id_fkey FOREIGN KEY (device_id) REFERENCES device (id) ON DELETE CASCADE;
