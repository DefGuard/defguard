CREATE TYPE device_state AS ENUM ('enabled', 'blocked');

ALTER TABLE device ADD COLUMN state device_state NOT NULL DEFAULT 'enabled';

CREATE TYPE device_verification AS ENUM ('verified', 'unverified', 'pending');

ALTER TABLE device ADD COLUMN verification device_verification NOT NULL DEFAULT 'unverified';
