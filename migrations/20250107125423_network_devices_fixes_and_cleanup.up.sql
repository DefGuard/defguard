-- Network devices
CREATE TYPE device_type AS ENUM (
    'user',
    'network'
);

ALTER TABLE device ADD COLUMN device_type device_type DEFAULT 'user'::device_type NOT NULL;
ALTER TABLE device ADD COLUMN description TEXT;
ALTER TABLE device ADD COLUMN configured BOOLEAN DEFAULT TRUE NOT NULL;
ALTER TABLE token ADD COLUMN device_id bigint;
ALTER TABLE token ADD CONSTRAINT enrollment_device_id_fkey FOREIGN KEY (device_id) REFERENCES device (id) ON DELETE CASCADE;
ALTER TABLE device DROP CONSTRAINT name_user;
ALTER TABLE device ADD CONSTRAINT name_user UNIQUE (name, user_id, device_type);

-- Web3 purge
ALTER TABLE session DROP web3_challenge;
DROP TABLE wallet;

CREATE TYPE mfa_method_new AS ENUM (
    'none',
    'one_time_password',
    'webauthn',
    'email'
);
UPDATE "user" SET mfa_method = 'none' WHERE mfa_method = 'web3';
ALTER TABLE "user"
    ALTER COLUMN mfa_method DROP DEFAULT,
    ALTER COLUMN mfa_method TYPE mfa_method_new USING mfa_method::TEXT::mfa_method_new,
    ALTER COLUMN mfa_method SET DEFAULT 'none'::mfa_method_new;
DROP TYPE mfa_method;
ALTER TYPE mfa_method_new RENAME TO mfa_method;

-- Stat fixes
CREATE OR REPLACE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload, 1, 0::bigint) OVER (PARTITION BY device_id ORDER BY collected_at), 0) upload,
        greatest(download - lag(download, 1, 0::bigint) OVER (PARTITION BY device_id ORDER BY collected_at), 0) download,
        latest_handshake - (lag(latest_handshake, 1, latest_handshake) OVER (PARTITION BY device_id ORDER BY collected_at)) latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;


-- Multiple network addresses
ALTER TABLE wireguard_network ALTER address TYPE inet[] USING ARRAY[address];
