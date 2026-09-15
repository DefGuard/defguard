ALTER TABLE vpn_client_mfa_session
    DROP COLUMN flow_kind;

DROP TYPE vpn_mfa_flow_kind;
