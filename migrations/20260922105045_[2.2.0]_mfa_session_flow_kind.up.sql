CREATE TYPE vpn_mfa_flow_kind AS ENUM (
    'legacy',
    'multi_step'
);

-- Existing MFA sessions belong to the legacy single-step flow.
ALTER TABLE vpn_client_mfa_session
    ADD COLUMN flow_kind vpn_mfa_flow_kind NOT NULL DEFAULT 'legacy';

ALTER TABLE vpn_client_mfa_session
    ALTER COLUMN flow_kind DROP DEFAULT;
