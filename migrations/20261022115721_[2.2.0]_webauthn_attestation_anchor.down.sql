DROP INDEX webauthn_attestation_chain_sha256_idx;
DROP INDEX webauthn_attestation_aaguid_idx;

ALTER TABLE webauthn
    DROP COLUMN attestation_aaguid,
    DROP COLUMN attestation_valid_until,
    DROP COLUMN attestation_chain,
    DROP COLUMN attestation_chain_sha256,
    DROP COLUMN registered_at;
