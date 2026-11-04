ALTER TABLE device_posture
    ADD COLUMN desktop_attestation boolean NOT NULL DEFAULT false,
    ADD COLUMN desktop_manual_verification boolean NOT NULL DEFAULT false,
    ADD COLUMN mobile_attestation boolean NOT NULL DEFAULT false,
    ADD COLUMN mobile_manual_verification boolean NOT NULL DEFAULT false;
