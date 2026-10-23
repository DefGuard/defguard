-- Existing keys stay NULL, their registration time and attestation were never recorded.
ALTER TABLE webauthn
    ADD COLUMN attestation_aaguid uuid NULL,
    ADD COLUMN attestation_root_sha256 text NULL,
    ADD COLUMN attestation_valid_until timestamp without time zone NULL,
    ADD COLUMN attestation_chain bytea[] NULL,
    ADD COLUMN attestation_chain_sha256 text[] NULL,
    ADD COLUMN registered_at timestamp without time zone NULL;

CREATE INDEX webauthn_attestation_aaguid_idx ON webauthn (attestation_aaguid);
CREATE INDEX webauthn_attestation_root_sha256_idx ON webauthn (attestation_root_sha256);
CREATE INDEX webauthn_attestation_chain_sha256_idx ON webauthn USING gin (attestation_chain_sha256);
