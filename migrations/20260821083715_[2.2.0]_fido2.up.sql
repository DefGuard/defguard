-- Web login MFA method (`MFAMethod`).
ALTER TYPE mfa_method ADD VALUE 'fido2';

-- Desktop/mobile client MFA method (`VpnClientMfaMethod`). Backs both
-- `vpn_client_session.mfa_method` and `mfa_flow_step.methods`, so the MFA flow editor cannot
-- store a FIDO2 step without it.
ALTER TYPE vpn_client_mfa_method ADD VALUE 'fido2';
