DROP TABLE wizard;
DROP TYPE active_wizard;

DROP TABLE mail_context;

DROP TABLE vpn_session_stats;
DROP TABLE vpn_client_session;
DROP TYPE vpn_client_mfa_method;
DROP TYPE vpn_client_session_state;

DROP TABLE proxy;

DROP TRIGGER gateway ON gateway;
DROP FUNCTION row_change();
DROP TABLE gateway;

ALTER TABLE aclrule
    DROP COLUMN any_address,
    DROP COLUMN any_port,
    DROP COLUMN any_protocol,
    DROP COLUMN use_manual_destination_settings,
    DROP COLUMN allow_all_groups,
    DROP COLUMN deny_all_groups;

ALTER TABLE aclrule RENAME COLUMN addresses TO destination;
ALTER TABLE aclrule RENAME COLUMN all_locations TO all_networks;

ALTER TABLE aclalias
    DROP COLUMN any_address,
    DROP COLUMN any_port,
    DROP COLUMN any_protocol;

ALTER TABLE aclalias RENAME COLUMN addresses TO destination;

ALTER TABLE openidprovider DROP COLUMN kind;
DROP TYPE openid_provider_kind;

ALTER TABLE wireguard_network
    DROP COLUMN mtu,
    DROP COLUMN fwmark;

ALTER TABLE settings DROP CONSTRAINT fk_default_admin;

ALTER TABLE settings
    DROP COLUMN ca_key_der,
    DROP COLUMN ca_cert_der,
    DROP COLUMN ca_expiry,
    DROP COLUMN defguard_url,
    DROP COLUMN default_admin_group_name,
    DROP COLUMN authentication_period_days,
    DROP COLUMN mfa_code_timeout_seconds,
    DROP COLUMN public_proxy_url,
    DROP COLUMN default_admin_id,
    DROP COLUMN secret_key,
    DROP COLUMN openid_signing_key,
    DROP COLUMN webauthn_rp_id,
    DROP COLUMN disable_stats_purge,
    DROP COLUMN stats_purge_frequency_hours,
    DROP COLUMN stats_purge_threshold_days,
    DROP COLUMN enrollment_token_timeout_hours,
    DROP COLUMN password_reset_token_timeout_hours,
    DROP COLUMN enrollment_session_timeout_minutes,
    DROP COLUMN password_reset_session_timeout_minutes;

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

CREATE INDEX peer_stats_device_id_collected_at
    ON wireguard_peer_stats (device_id, network, collected_at DESC, latest_handshake DESC NULLS LAST);

CREATE OR REPLACE VIEW wireguard_peer_stats_view AS
    SELECT
        device_id,
        greatest(upload - lag(upload, 1, upload) OVER (PARTITION BY device_id, network ORDER BY collected_at), 0) upload,
        greatest(download - lag(download, 1, download) OVER (PARTITION BY device_id, network ORDER BY collected_at), 0) download,
        latest_handshake - (lag(latest_handshake, 1, latest_handshake) OVER (PARTITION BY device_id, network ORDER BY collected_at)) latest_handshake_diff,
        latest_handshake,
        collected_at,
        network,
        endpoint,
        allowed_ips
    FROM wireguard_peer_stats;
