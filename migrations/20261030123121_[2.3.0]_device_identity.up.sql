CREATE TYPE device_attestation_status AS ENUM (
    'not_attested', 'attested', 'unrecognized', 'attested_manually', 'attestation_failed'
);

ALTER TABLE device
    ADD COLUMN attestation_status device_attestation_status NOT NULL DEFAULT 'not_attested',
    ADD COLUMN attested_at timestamp without time zone NULL;
