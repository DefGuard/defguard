ALTER TABLE wireguard_network_device
    ADD COLUMN preshared_key text NULL,
    ADD COLUMN is_authorized bool NOT NULL DEFAULT false,
    ADD COLUMN authorized_at timestamp without time zone NULL;

-- Rollback is lossy: only preshared_key is repopulated from the latest active
-- session per (device_id, location_id); is_authorized and authorized_at are
-- recreated with default/NULL values and are not reconstructed.
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

DROP INDEX IF EXISTS vpn_client_session_active_location_device_unique;

ALTER TABLE vpn_client_session DROP COLUMN preshared_key;
