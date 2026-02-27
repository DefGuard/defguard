ALTER TABLE settings
    ADD COLUMN auth_cookie_timeout interval DEFAULT interval '7 days',
    ADD COLUMN secret_key text DEFAULT 'UNSET', -- TODO(jck)
    ADD COLUMN grpc_ca text,
    ADD COLUMN grpc_cert text,
    ADD COLUMN grpc_key text, ADD COLUMN openid_signing_key text,
    ADD COLUMN webauthn_rp_id text,
    ADD COLUMN grpc_url text DEFAULT 'http://localhost:50055',
    ADD COLUMN disable_stats_purge boolean DEFAULT false,
    ADD COLUMN stats_purge_frequency interval DEFAULT interval '24 hours',
    ADD COLUMN stats_purge_threshold interval DEFAULT interval '30 days',
    ADD COLUMN enrollment_token_timeout interval DEFAULT interval '24 hours',
    ADD COLUMN password_reset_token_timeout interval DEFAULT interval '24 hours',
    ADD COLUMN enrollment_session_timeout interval DEFAULT interval '10 minutes',
    ADD COLUMN password_reset_session_timeout interval DEFAULT interval '10 minutes',
    ADD COLUMN proxy_grpc_ca text;
