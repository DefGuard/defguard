CREATE TYPE service_location_mode AS ENUM (
    'disabled',
    'prelogon',
    'alwayson'
);

ALTER TABLE wireguard_network ADD COLUMN "service_location_mode" service_location_mode NOT NULL DEFAULT 'disabled';
