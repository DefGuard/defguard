DROP TABLE device_identity_challenge;
DROP TABLE device_attestation_state;
ALTER TABLE vpn_client_mfa_session
    DROP CONSTRAINT vpn_client_mfa_session_identity_context,
    DROP COLUMN possession_verified;
DROP TABLE device_identity;
DROP TYPE challenge_purpose;
DROP TYPE device_identity_backend;
