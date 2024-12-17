CREATE TYPE device_type AS ENUM (
    'user',
    'network'
);

ALTER TABLE device ADD COLUMN device_type device_type DEFAULT 'user'::device_type NOT NULL;
ALTER TABLE device ADD COLUMN description TEXT;
