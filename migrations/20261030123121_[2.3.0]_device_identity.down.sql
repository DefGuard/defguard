DROP TABLE device_identity_challenge;
ALTER TABLE token DROP COLUMN attestation_state;
ALTER TABLE vpn_client_mfa_session DROP COLUMN possession_state;
ALTER TABLE device
    DROP CONSTRAINT device_identity_backend_fields,
    DROP CONSTRAINT device_identity_apple_counter_range,
    DROP CONSTRAINT device_identity_attested_key,
    DROP COLUMN identity_backend,
    DROP COLUMN identity_public_key_der,
    DROP COLUMN attestation_status,
    DROP COLUMN attested_at,
    DROP COLUMN apple_sign_count,
    DROP COLUMN tpm_ek_public,
    DROP COLUMN tpm_ek_certificate_chain_der,
    DROP COLUMN apple_key_id;
DROP TYPE challenge_purpose;
DROP TYPE device_attestation_status;
DROP TYPE device_identity_backend;
