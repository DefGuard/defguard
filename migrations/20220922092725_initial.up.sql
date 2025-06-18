CREATE TABLE authorization_code (
    id bigserial PRIMARY KEY,
    "user" text NOT NULL,
    client_id text NOT NULL,
    code text NOT NULL UNIQUE,
    redirect_uri text NOT NULL,
    scope text NOT NULL,
    auth_time bigint NOT NULL
);
CREATE TABLE authorizedapps (
    id bigserial PRIMARY KEY,
    username text NOT NULL,
    client_id text NOT NULL,
    home_url text NOT NULL,
    date text NOT NULL
);
CREATE TABLE "user" (
    id bigserial PRIMARY KEY,
    username text UNIQUE NOT NULL,
    password_hash text NOT NULL,
    last_name text NOT NULL,
    first_name text NOT NULL,
    email text NOT NULL,
    phone text NULL,
    ssh_key text NULL,
    pgp_key text NULL,
    pgp_cert_id text NULL,
    totp_enabled boolean NOT NULL DEFAULT false,
    totp_secret bytea NULL
);
CREATE TABLE "device" (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    wireguard_ip text NOT NULL,
    wireguard_pubkey text NOT NULL,
    user_id bigint NOT NULL,
    created timestamp without time zone NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id),
    CONSTRAINT name_user UNIQUE (name, user_id)
);
CREATE TABLE "group" (id bigserial PRIMARY KEY, name text UNIQUE NOT NULL);
CREATE TABLE "group_user" (
    group_id bigint REFERENCES "group"(id) ON DELETE CASCADE,
    user_id bigint REFERENCES "user"(id) ON DELETE CASCADE,
    CONSTRAINT group_user_unique UNIQUE (group_id, user_id)
);
CREATE TABLE oauth2client (
    id bigserial PRIMARY KEY,
    "user" text NOT NULL,
    client_id text NOT NULL UNIQUE,
    client_secret text NOT NULL,
    redirect_uri text NOT NULL,
    scope text NOT NULL
);
CREATE TABLE oauth2token (
    id bigserial PRIMARY KEY,
    access_token text NOT NULL UNIQUE,
    refresh_token text NOT NULL UNIQUE,
    redirect_uri text NOT NULL,
    scope text NOT NULL,
    expires_in bigint NOT NULL
);
CREATE TABLE openidclient (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    description text NOT NULL,
    home_url text NOT NULL UNIQUE,
    client_id text NOT NULL UNIQUE,
    client_secret text NOT NULL UNIQUE,
    redirect_uri text NOT NULL,
    enabled boolean NOT NULL DEFAULT true
);
CREATE TABLE openidclientauthcode (
    id bigserial PRIMARY KEY,
    "user" text NOT NULL,
    code text NOT NULL UNIQUE,
    client_id text NOT NULL UNIQUE,
    state text NOT NULL UNIQUE,
    scope text NOT NULL,
    redirect_uri text NOT NULL,
    nonce text
);
CREATE TABLE settings (
    id bigserial PRIMARY KEY,
    web3_enabled boolean NOT NULL,
    openid_enabled boolean NOT NULL,
    oauth_enabled boolean NOT NULL,
    ldap_enabled boolean NOT NULL,
    wireguard_enabled boolean NOT NULL,
    webhooks_enabled boolean NOT NULL,
    worker_enabled boolean NOT NULL,
    challenge_template text NOT NULL
);
CREATE TABLE "wallet" (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    address text NOT NULL UNIQUE,
    challenge_message text NOT NULL,
    challenge_signature text NULL,
    creation_timestamp timestamp without time zone NOT NULL,
    validation_timestamp timestamp without time zone NULL,
    name text NOT NULL DEFAULT '',
    chain_id bigint NOT NULL DEFAULT 0
);
CREATE TABLE webhook (
    id bigserial PRIMARY KEY,
    url text NOT NULL UNIQUE,
    description text NOT NULL,
    token text NOT NULL,
    enabled boolean NOT NULL,
    on_user_created boolean NOT NULL DEFAULT false,
    on_user_deleted boolean NOT NULL DEFAULT false,
    on_user_modified boolean NOT NULL DEFAULT false,
    on_hwkey_provision boolean NOT NULL DEFAULT false
);
CREATE TABLE wireguard_network (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    address text NOT NULL,
    port integer NOT NULL,
    pubkey text NOT NULL,
    prvkey text NOT NULL,
    endpoint text NOT NULL,
    dns text,
    allowed_ips text,
    connected_at timestamp without time zone
);
CREATE TABLE wireguard_peer_stats (
    id bigserial PRIMARY KEY,
    device_id bigint NOT NULL,
    collected_at timestamp without time zone NOT NULL DEFAULT current_timestamp,
    network bigint NOT NULL,
    endpoint text,
    upload bigint NOT NULL,
    download bigint NOT NULL,
    latest_handshake timestamp without time zone NOT NULL,
    allowed_ips text,
    FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE CASCADE
);
CREATE TABLE session (
    id text PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    state smallint NOT NULL,
    created timestamp without time zone NOT NULL,
    expires timestamp without time zone NOT NULL,
    webauthn_challenge bytea NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id)
);
CREATE TABLE webauthn (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL,
    passkey bytea NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id)
);
CREATE INDEX peer_stats_device_id_collected_at on wireguard_peer_stats (device_id, collected_at);
CREATE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload) OVER (PARTITION BY device_id ORDER BY collected_at), 0) AS upload,
        greatest(download - lag(download) OVER (PARTITION BY device_id ORDER BY collected_at), 0) AS download,
        (latest_handshake - (lag(latest_handshake) OVER (PARTITION BY device_id ORDER BY collected_at))) AS latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;
INSERT INTO
    "group" (name)
VALUES
    ('admin');
INSERT INTO settings VALUES (
    1, true, true, true, true, true, true, true,
    'By signing this message you confirm that you''re the owner of the wallet'
);
