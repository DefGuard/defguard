-- Add session-level preshared_key, enforce at most one active session per
-- (location_id, device_id), and drop device-level preshared_key/auth fields.
-- WARNING: rollback is lossy for dropped wireguard_network_device columns.
-- Do not yet require preshared_key for active MFA sessions.
ALTER TABLE vpn_client_session ADD COLUMN preshared_key text NULL;

CREATE UNIQUE INDEX vpn_client_session_active_location_device_unique
    ON vpn_client_session(location_id, device_id)
    WHERE state IN ('new', 'connected');

ALTER TABLE wireguard_network_device
    DROP COLUMN preshared_key,
    DROP COLUMN is_authorized,
    DROP COLUMN authorized_at;
