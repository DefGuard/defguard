ALTER TABLE wireguard_network_device
    ADD COLUMN preshared_key text NULL,
    ADD COLUMN is_authorized bool NOT NULL DEFAULT false,
    ADD COLUMN authorized_at timestamp without time zone NULL;

-- Restore legacy preshared keys when canonical active session data exists; unmatched rows stay NULL.
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

ALTER TABLE vpn_client_session DROP COLUMN preshared_key;
