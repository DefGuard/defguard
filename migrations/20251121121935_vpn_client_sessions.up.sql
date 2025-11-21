CREATE TYPE vpn_client_session_state AS ENUM (
    'new',
    'connected',
    'disconnected'
);

CREATE TABLE vpn_client_session (
    id bigserial PRIMARY KEY,
    location_id bigint NOT NULL,
    user_id bigint NOT NULL,
    device_id bigint NULL,
    created_at timestamp without time zone NOT NULL DEFAULT current_timestamp,
    connected_at timestamp without time zone NOT NULL,
    disconnected_at timestamp without time zone NOT NULL,
    mfa boolean NOT NULL,
    state vpn_client_session_state NOT NULL DEFAULT 'new',
    FOREIGN KEY (location_id) REFERENCES wireguard_network(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE SET NULL
);

CREATE TABLE vpn_session_stats (
    id bigserial PRIMARY KEY,
    session_id bigint NOT NULL,
    upload_diff bigint NOT NULL,
    download_diff bigint NOT NULL,
    FOREIGN KEY (session_id) REFERENCES vpn_client_session(id) ON DELETE CASCADE
);
