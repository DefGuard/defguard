CREATE TYPE vpn_client_session_state AS ENUM (
    'new',
    'connected',
    'disconnected'
);

CREATE TABLE vpn_client_session (
    id bigserial PRIMARY KEY,
    location_id bigint NOT NULL,
    user_id bigint NOT NULL,
    device_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT current_timestamp,
    connected_at timestamp without time zone NULL,
    disconnected_at timestamp without time zone NULL,
    mfa_mode location_mfa_mode NOT NULL,
    state vpn_client_session_state NOT NULL DEFAULT 'new',
    FOREIGN KEY (location_id) REFERENCES wireguard_network(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE CASCADE
);
CREATE INDEX idx_vpn_client_session_user_id ON vpn_client_session(user_id);
CREATE INDEX idx_vpn_client_session_device_id ON vpn_client_session(device_id);
CREATE INDEX idx_vpn_client_session_location_id ON vpn_client_session(location_id);
CREATE INDEX idx_vpn_client_session_state ON vpn_client_session(state);
CREATE INDEX idx_vpn_client_session_created_at ON vpn_client_session(created_at DESC);
CREATE INDEX idx_vpn_client_session_connected_at ON vpn_client_session(connected_at DESC);

CREATE TABLE vpn_session_stats (
    id bigserial PRIMARY KEY,
    session_id bigint NOT NULL,
    gateway_id bigint NOT NULL,
    collected_at timestamp without time zone NOT NULL,
    latest_handshake timestamp without time zone NOT NULL,
    endpoint text NOT NULL,
    total_upload bigint NOT NULL,
    total_download bigint NOT NULL,
    upload_diff bigint NOT NULL,
    download_diff bigint NOT NULL,
    FOREIGN KEY (session_id) REFERENCES vpn_client_session(id) ON DELETE CASCADE,
    FOREIGN KEY (gateway_id) REFERENCES gateway(id) ON DELETE CASCADE
);
CREATE INDEX idx_vpn_session_stats_session_id ON vpn_session_stats(session_id);
CREATE INDEX idx_vpn_session_stats_gateway_id ON vpn_session_stats(gateway_id);
CREATE INDEX idx_vpn_session_stats_collected_at ON vpn_session_stats(collected_at DESC);
CREATE INDEX idx_vpn_session_stats_latest_handshake ON vpn_session_stats(latest_handshake DESC);
CREATE INDEX idx_vpn_session_stats_session_collected ON vpn_session_stats(session_id, collected_at DESC);
