ALTER TABLE gateway
    DROP COLUMN address,
    DROP COLUMN port,
    DROP COLUMN modified_at,
    DROP COLUMN modified_by,
    ADD COLUMN hostname TEXT NOT NULL DEFAULT 'gateway',
    ADD COLUMN url text NOT NULL DEFAULT 'http://127.0.0.1:50051';

ALTER TABLE gateway RENAME COLUMN location_id TO network_id;

