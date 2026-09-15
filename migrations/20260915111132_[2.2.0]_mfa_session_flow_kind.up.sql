CREATE TYPE vpn_mfa_flow_kind AS ENUM (
    'legacy',
    'multi_step'
);

-- In-progress MFA sessions are disposable, so no pre-marker rows need a value.
DELETE FROM vpn_client_mfa_session;

ALTER TABLE vpn_client_mfa_session
    ADD COLUMN flow_kind vpn_mfa_flow_kind NOT NULL;
