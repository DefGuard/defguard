-- Transitional additive step: add session-level preshared_key now; application rollout moves data later.
ALTER TABLE vpn_client_session ADD COLUMN preshared_key text NULL;
