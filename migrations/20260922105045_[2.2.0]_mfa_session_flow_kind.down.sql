ALTER TABLE vpn_client_mfa_session
    DROP COLUMN IF EXISTS flow_kind;

DROP TYPE IF EXISTS vpn_mfa_flow_kind;
