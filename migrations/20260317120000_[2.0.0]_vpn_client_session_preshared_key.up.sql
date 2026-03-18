-- Transitional squash: add session-level preshared_key and remove legacy device-level MFA state.
ALTER TABLE vpn_client_session ADD COLUMN preshared_key text NULL;

ALTER TABLE wireguard_network_device
    DROP COLUMN preshared_key,
    DROP COLUMN is_authorized,
    DROP COLUMN authorized_at;
