CREATE TYPE device_identity_backend AS ENUM ('tpm', 'apple_app_attest');
CREATE TYPE device_attestation_status AS ENUM (
    'not_attested', 'attested', 'unrecognized', 'attested_manually'
);

ALTER TABLE device
    ADD COLUMN identity_backend device_identity_backend NULL,
    ADD COLUMN identity_public_key_der bytea NULL,
    ADD COLUMN attestation_status device_attestation_status NOT NULL DEFAULT 'not_attested',
    ADD COLUMN attested_at timestamp without time zone NULL,
    ADD COLUMN apple_sign_count bigint NULL;

ALTER TABLE device
    ADD CONSTRAINT device_identity_backend_fields CHECK (
        (identity_backend IS NOT NULL OR (identity_public_key_der IS NULL AND apple_sign_count IS NULL))
        AND (apple_sign_count IS NULL OR identity_backend = 'apple_app_attest')
    ),
    ADD CONSTRAINT device_identity_apple_counter_range
        CHECK (apple_sign_count IS NULL OR apple_sign_count BETWEEN 0 AND 4294967295),
    ADD CONSTRAINT device_identity_attested_key CHECK (
        attestation_status NOT IN ('attested', 'attested_manually')
        OR (identity_backend IS NOT NULL AND identity_public_key_der IS NOT NULL AND attested_at IS NOT NULL)
    );

ALTER TABLE token ADD COLUMN attestation_state bytea NULL;
ALTER TABLE vpn_client_mfa_session ADD COLUMN possession_state jsonb NULL;

CREATE TYPE challenge_purpose AS ENUM ('polling', 'posture_connection');

CREATE TABLE device_identity_challenge (
    id bigserial PRIMARY KEY,
    purpose challenge_purpose NOT NULL,
    device_id bigint NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    location_id bigint NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    data bytea NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    expires_at timestamp without time zone NOT NULL,
    consumed_at timestamp without time zone NULL,
    CONSTRAINT device_identity_challenge_location CHECK (
        (purpose = 'polling' AND location_id IS NULL)
        OR (purpose = 'posture_connection' AND location_id IS NOT NULL)
    ),
    CONSTRAINT device_identity_challenge_data CHECK (octet_length(data) > 0),
    CONSTRAINT device_identity_challenge_expiry CHECK (expires_at > created_at),
    CONSTRAINT device_identity_challenge_consumption
        CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX device_identity_challenge_device_idx ON device_identity_challenge (device_id);
CREATE INDEX device_identity_challenge_expiry_idx ON device_identity_challenge (expires_at);
