-- Holds the in-progress WebAuthn PasskeyRegistration (CBOR) between the FIDO2
-- CodeMfaSetup Start and Finish calls; analogous to `session.webauthn_challenge`.
ALTER TABLE token ADD COLUMN mfa_setup_state bytea;
