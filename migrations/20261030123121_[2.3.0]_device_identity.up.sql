CREATE TYPE device_identity_backend AS ENUM ('tpm', 'apple_app_attest');
CREATE TYPE device_attestation_status AS ENUM (
    'not_attested', 'attested', 'unrecognized', 'attested_manually'
);
CREATE TYPE device_identity_error_code AS ENUM (
    'unavailable', 'temporarily_unavailable', 'timeout', 'rate_limited',
    'permission_denied', 'unsupported', 'identity_lost', 'invalid_challenge',
    'backend_failure', 'invalid_evidence', 'challenge_expired'
);
CREATE TYPE challenge_purpose AS ENUM ('attestation', 'possession');

CREATE TABLE device_identity (
    id uuid PRIMARY KEY,
    device_id bigint NOT NULL UNIQUE REFERENCES device(id) ON DELETE CASCADE,
    backend device_identity_backend NOT NULL,
    public_key_der bytea NULL,
    key_fingerprint bytea NULL,
    ek_fingerprint bytea NULL,
    apple_key_id text NULL,
    apple_sign_count bigint NULL,
    attestation_status device_attestation_status NOT NULL DEFAULT 'not_attested',
    error_code device_identity_error_code NULL,
    capabilities jsonb NULL,
    attestation_evidence bytea NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    attested_at timestamp without time zone NULL,
    last_possession_verified_at timestamp without time zone NULL,
    UNIQUE (id, device_id),
    CONSTRAINT device_identity_key_fingerprint_length
        CHECK (key_fingerprint IS NULL OR octet_length(key_fingerprint) = 32),
    CONSTRAINT device_identity_ek_fingerprint_length
        CHECK (ek_fingerprint IS NULL OR octet_length(ek_fingerprint) = 32),
    CONSTRAINT device_identity_public_key_fields
        CHECK ((public_key_der IS NULL) = (key_fingerprint IS NULL)),
    CONSTRAINT device_identity_apple_counter_range
        CHECK (apple_sign_count IS NULL OR apple_sign_count BETWEEN 0 AND 4294967295),
    CONSTRAINT device_identity_backend_fields CHECK (
        (backend = 'tpm' AND apple_key_id IS NULL AND apple_sign_count IS NULL)
        OR (backend = 'apple_app_attest' AND ek_fingerprint IS NULL)
    ),
    CONSTRAINT device_identity_attested_key CHECK (
        attestation_status NOT IN ('attested', 'attested_manually')
        OR (public_key_der IS NOT NULL AND attested_at IS NOT NULL)
    )
);

CREATE TABLE device_identity_challenge (
    id uuid PRIMARY KEY,
    purpose challenge_purpose NOT NULL,
    device_id bigint NULL REFERENCES device(id) ON DELETE CASCADE,
    identity_id uuid NULL,
    enrollment_token_id text NULL REFERENCES token(id) ON DELETE CASCADE,
    data bytea NOT NULL,
    credential_blob bytea NULL,
    encrypted_secret bytea NULL,
    activation_secret_hash bytea NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    expires_at timestamp without time zone NOT NULL,
    consumed_at timestamp without time zone NULL,
    CONSTRAINT device_identity_challenge_identity_device_fkey
        FOREIGN KEY (identity_id, device_id) REFERENCES device_identity(id, device_id)
        ON DELETE CASCADE,
    CONSTRAINT device_identity_challenge_owner
        CHECK (num_nonnulls(device_id, enrollment_token_id) = 1),
    CONSTRAINT device_identity_challenge_identity_owner
        CHECK (identity_id IS NULL OR device_id IS NOT NULL),
    CONSTRAINT device_identity_challenge_credential_fields
        CHECK (num_nonnulls(credential_blob, encrypted_secret, activation_secret_hash) IN (0, 3)),
    CONSTRAINT device_identity_challenge_purpose_fields CHECK (
        purpose = 'attestation'
        OR (identity_id IS NOT NULL AND credential_blob IS NULL)
    ),
    CONSTRAINT device_identity_challenge_secret_hash_length
        CHECK (activation_secret_hash IS NULL OR octet_length(activation_secret_hash) = 32),
    CONSTRAINT device_identity_challenge_data CHECK (octet_length(data) > 0),
    CONSTRAINT device_identity_challenge_expiry CHECK (expires_at > created_at),
    CONSTRAINT device_identity_challenge_consumption
        CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX device_identity_challenge_device_idx ON device_identity_challenge (device_id);
CREATE INDEX device_identity_challenge_token_idx ON device_identity_challenge (enrollment_token_id);
CREATE INDEX device_identity_challenge_expiry_idx ON device_identity_challenge (expires_at);
