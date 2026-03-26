-- Drop wizard state introduced in 2.0.0.
DROP TABLE wizard;
DROP TYPE active_wizard;

-- Remove database-backed mail templates.
DROP TABLE mail_context;

UPDATE activity_log_event
SET ip = '0.0.0.0'::inet
WHERE ip IS NULL;

ALTER TABLE activity_log_event ALTER COLUMN ip SET NOT NULL;

ALTER TABLE wireguard_network_device
    ADD COLUMN preshared_key text NULL,
    ADD COLUMN is_authorized bool NOT NULL DEFAULT false,
    ADD COLUMN authorized_at timestamp without time zone NULL;

-- Rollback is lossy: only preshared_key is repopulated from an active
-- session with a non-null preshared_key per (device_id, location_id);
-- is_authorized and authorized_at are recreated with default/NULL values and
-- are not reconstructed.
UPDATE wireguard_network_device AS network_device
SET preshared_key = latest_active_session.preshared_key
FROM (
    SELECT DISTINCT ON (session.device_id, session.location_id)
        session.device_id,
        session.location_id,
        session.preshared_key
    FROM vpn_client_session AS session
    WHERE session.state IN ('new', 'connected')
      AND session.preshared_key IS NOT NULL
    ORDER BY session.device_id, session.location_id, session.created_at DESC, session.id DESC
) AS latest_active_session
WHERE network_device.device_id = latest_active_session.device_id
  AND network_device.wireguard_network_id = latest_active_session.location_id;

-- Remove VPN session tracking and proxy management structures.
DROP TABLE vpn_session_stats;
DROP TABLE vpn_client_session;
DROP TYPE vpn_client_mfa_method;
DROP TYPE vpn_client_session_state;

DROP TABLE proxy;

DROP TRIGGER gateway ON gateway;
DROP FUNCTION row_change();
DROP TABLE gateway;

-- Restore ACL naming and flags from before 2.0.0.
ALTER TABLE aclrule RENAME COLUMN addresses TO destination;
ALTER TABLE aclrule RENAME COLUMN all_locations TO all_networks;

ALTER TABLE aclalias RENAME COLUMN addresses TO destination;

ALTER TABLE aclrule
    DROP COLUMN any_address,
    DROP COLUMN any_port,
    DROP COLUMN any_protocol,
    DROP COLUMN use_manual_destination_settings,
    DROP COLUMN allow_all_groups,
    DROP COLUMN deny_all_groups;

ALTER TABLE aclalias
    DROP COLUMN any_address,
    DROP COLUMN any_port,
    DROP COLUMN any_protocol;

-- Remove 2.0.0 OpenID provider extensions.
ALTER TABLE openidprovider DROP COLUMN kind;
DROP TYPE openid_provider_kind;

-- Remove 2.0.0 WireGuard network defaults.
ALTER TABLE wireguard_network
    DROP COLUMN mtu,
    DROP COLUMN fwmark,
    DROP COLUMN allow_all_groups;

-- Remove 2.0.0 setup and settings columns.
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
    DROP COLUMN enable_stats_purge,
    DROP COLUMN stats_purge_frequency_hours,
    DROP COLUMN stats_purge_threshold_days,
    DROP COLUMN enrollment_token_timeout_hours,
    DROP COLUMN enrollment_send_welcome_email,
    DROP COLUMN password_reset_token_timeout_hours,
    DROP COLUMN enrollment_session_timeout_minutes,
    DROP COLUMN password_reset_session_timeout_minutes;

-- Restore the legacy peer stats structures used before 2.0.0.
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
