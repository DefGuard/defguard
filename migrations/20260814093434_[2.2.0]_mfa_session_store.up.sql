-- Authorized session records ONLY whether it was MFA-gated. The methods used and the
-- governing flow live in the immutable authorization activity-log entry, not on this row.
ALTER TABLE vpn_client_session ADD COLUMN is_mfa_session boolean NOT NULL DEFAULT false;
UPDATE vpn_client_session SET is_mfa_session = true WHERE mfa_method IS NOT NULL;
ALTER TABLE vpn_client_session DROP COLUMN mfa_method;

-- Durable in-progress MFA session. Token is OPAQUE (random); only its hash is stored.
-- All per-step ephemeral state lives in `ephemeral_state` (JSONB), cleared to NULL on advance.
CREATE TABLE vpn_client_mfa_session (
    id              bigserial PRIMARY KEY,
    token_hash      text NOT NULL UNIQUE,   -- base64url-nopad SHA-256 of the opaque token; raw token never stored
    location_id     bigint NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    device_id       bigint NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    user_id         bigint NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,  -- denormalized; NOT part of the key
    steps_snapshot  jsonb NOT NULL,          -- {"flow_id": <id>, "steps": [{"methods": [...], "satisfied": <method>|null}, ...]}
    current_step    integer NOT NULL DEFAULT 0,
    ephemeral_state jsonb NULL,              -- per-step attempt state; cleared on advance
    failed_attempts integer NOT NULL DEFAULT 0,
    created_at      timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at      timestamp without time zone NOT NULL
);

-- The (location_id, device_id) identity is enforced by construction: a concurrent double-Start
-- cannot leave two live rows, because `start` supersedes via a single-statement upsert.
CREATE UNIQUE INDEX vpn_client_mfa_session_location_device_unique
    ON vpn_client_mfa_session (location_id, device_id);
