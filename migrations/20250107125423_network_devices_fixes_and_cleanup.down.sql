ALTER TABLE device
DROP COLUMN device_type;

ALTER TABLE device
DROP COLUMN description;

ALTER TABLE device
DROP COLUMN configured;

ALTER TABLE token
DROP CONSTRAINT enrollment_device_id_fkey;

ALTER TABLE token
DROP COLUMN device_id;

DROP TYPE device_type;

ALTER TYPE mfa_method ADD VALUE 'web3';
CREATE TABLE wallet (
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
ALTER TABLE session ADD web3_challenge text NULL;

CREATE OR REPLACE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload) OVER (PARTITION BY device_id ORDER BY collected_at), 0) upload,
        greatest(download - lag(download) OVER (PARTITION BY device_id ORDER BY collected_at), 0) download,
        latest_handshake - (lag(latest_handshake) OVER (PARTITION BY device_id ORDER BY collected_at)) latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;

ALTER TABLE wireguard_network ALTER address TYPE inet USING address[1];

ALTER TABLE device DROP CONSTRAINT name_user;
ALTER TABLE device ADD CONSTRAINT name_user UNIQUE (name, user_id);
