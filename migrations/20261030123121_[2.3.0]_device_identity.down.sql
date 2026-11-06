ALTER TABLE device
    DROP COLUMN attested_at,
    DROP COLUMN attestation_status;

DROP TYPE device_attestation_status;
