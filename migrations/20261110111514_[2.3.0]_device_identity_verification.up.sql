CREATE TYPE device_identity_backend AS ENUM ('tpm', 'apple_app_attest');
ALTER TABLE device
    ADD COLUMN identity_backend device_identity_backend NULL,
    ADD COLUMN identity_public_key_der bytea NULL,
    ADD COLUMN apple_sign_count bigint NULL,
    ADD COLUMN tpm_ek_public bytea NULL,
    ADD COLUMN tpm_ek_certificate_chain_der bytea[] NULL,
    ADD COLUMN apple_key_id text NULL;

ALTER TABLE device
    ADD CONSTRAINT device_identity_backend_fields CHECK (
        (identity_backend IS NOT NULL OR (
            identity_public_key_der IS NULL AND apple_sign_count IS NULL
            AND tpm_ek_public IS NULL AND tpm_ek_certificate_chain_der IS NULL AND apple_key_id IS NULL
        ))
        AND ((tpm_ek_public IS NULL AND tpm_ek_certificate_chain_der IS NULL) OR identity_backend = 'tpm')
        AND ((apple_sign_count IS NULL AND apple_key_id IS NULL) OR identity_backend = 'apple_app_attest')
    ),
    ADD CONSTRAINT device_identity_apple_counter_range
        CHECK (apple_sign_count IS NULL OR apple_sign_count BETWEEN 0 AND 4294967295),
    ADD CONSTRAINT device_identity_attested_key CHECK (
        attestation_status NOT IN ('attested', 'attested_manually')
        OR (
            identity_backend IS NOT NULL AND identity_public_key_der IS NOT NULL AND attested_at IS NOT NULL
            AND (
                attestation_status = 'attested_manually'
                OR (identity_backend = 'tpm' AND tpm_ek_public IS NOT NULL AND tpm_ek_certificate_chain_der IS NOT NULL)
                OR (identity_backend = 'apple_app_attest' AND apple_key_id IS NOT NULL)
            )
        )
    );

CREATE TABLE device_attestation_state (
    challenge_id text PRIMARY KEY,
    token_id text NOT NULL UNIQUE REFERENCES token(id) ON DELETE CASCADE,
    wireguard_key text NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    expires_at timestamp without time zone NOT NULL,
    payload bytea NOT NULL,
    CONSTRAINT device_attestation_state_challenge_id CHECK (length(challenge_id) > 0),
    CONSTRAINT device_attestation_state_wireguard_key CHECK (length(wireguard_key) > 0),
    CONSTRAINT device_attestation_state_payload CHECK (octet_length(payload) > 0),
    CONSTRAINT device_attestation_state_expiry CHECK (expires_at > created_at)
);

CREATE INDEX device_attestation_state_expiry_idx ON device_attestation_state (expires_at);
ALTER TABLE vpn_client_mfa_session
    ADD COLUMN possession_verified boolean NOT NULL DEFAULT false,
    ADD CONSTRAINT vpn_client_mfa_session_identity_context UNIQUE (id, device_id, location_id);

CREATE TYPE challenge_purpose AS ENUM ('polling', 'posture_connection', 'mfa_connection');

CREATE TABLE device_identity_challenge (
    id bigserial PRIMARY KEY,
    purpose challenge_purpose NOT NULL,
    device_id bigint NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    location_id bigint NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    mfa_session_id bigint NULL,
    mfa_session_token_hash text NULL,
    data bytea NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    expires_at timestamp without time zone NOT NULL,
    consumed_at timestamp without time zone NULL,
    CONSTRAINT device_identity_challenge_location CHECK (
        (purpose = 'polling' AND location_id IS NULL AND mfa_session_id IS NULL AND mfa_session_token_hash IS NULL)
        OR (purpose = 'posture_connection' AND location_id IS NOT NULL AND mfa_session_id IS NULL AND mfa_session_token_hash IS NULL)
        OR (purpose = 'mfa_connection' AND location_id IS NOT NULL AND mfa_session_id IS NOT NULL AND mfa_session_token_hash IS NOT NULL)
    ),
    CONSTRAINT device_identity_challenge_mfa_session FOREIGN KEY (mfa_session_id, device_id, location_id)
        REFERENCES vpn_client_mfa_session (id, device_id, location_id) ON DELETE CASCADE,
    CONSTRAINT device_identity_challenge_data CHECK (octet_length(data) > 0),
    CONSTRAINT device_identity_challenge_expiry CHECK (expires_at > created_at),
    CONSTRAINT device_identity_challenge_consumption
        CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX device_identity_challenge_device_idx ON device_identity_challenge (device_id);
CREATE INDEX device_identity_challenge_expiry_idx ON device_identity_challenge (expires_at);

CREATE INDEX device_identity_challenge_mfa_session_idx ON device_identity_challenge (mfa_session_id);
