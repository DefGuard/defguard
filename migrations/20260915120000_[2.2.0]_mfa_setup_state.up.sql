-- Holds the in-progress WebAuthn PasskeyRegistration (CBOR-serialized) between the
-- FIDO2 CodeMfaSetup Start and Finish calls. NULL for code-based methods and once a
-- ceremony completes. Analogous to `session.webauthn_challenge` on the REST path.
ALTER TABLE token ADD COLUMN mfa_setup_state bytea;
