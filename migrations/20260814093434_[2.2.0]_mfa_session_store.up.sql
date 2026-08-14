-- Authorized session records the full ordered method sequence + the governing flow.
ALTER TABLE vpn_client_session ADD COLUMN mfa_methods vpn_client_mfa_method[] NOT NULL DEFAULT '{}';
UPDATE vpn_client_session SET mfa_methods = ARRAY[mfa_method]::vpn_client_mfa_method[]
  WHERE mfa_method IS NOT NULL;
ALTER TABLE vpn_client_session DROP COLUMN mfa_method;

ALTER TABLE vpn_client_session ADD COLUMN flow_id bigint NULL
  REFERENCES mfa_flow(id) ON DELETE SET NULL;

-- Durable in-progress MFA session. Token is OPAQUE (random); only its hash is stored.
-- All per-step ephemeral state lives in `ephemeral_state` (JSONB), cleared to NULL on advance.
CREATE TABLE vpn_client_mfa_session (
    id              bigserial PRIMARY KEY,
    token_hash      text NOT NULL UNIQUE,   -- base64url-nopad SHA-256 of the opaque token; raw token never stored
    location_id     bigint NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    device_id       bigint NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    user_id         bigint NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,  -- denormalized; NOT part of the key
    steps_snapshot  jsonb NOT NULL,          -- {"flow_id": <id>, "steps": [{"methods": [...]}, ...]}
    current_step    integer NOT NULL DEFAULT 0,
    ephemeral_state jsonb NULL,              -- per-step attempt state; cleared on advance
    failed_attempts integer NOT NULL DEFAULT 0,
    created_at      timestamp without time zone NOT NULL DEFAULT current_timestamp,
    expires_at      timestamp without time zone NOT NULL
);

-- The (location_id, device_id) identity is enforced by construction: a concurrent double-Start
-- cannot leave two live rows, because `start` supersedes via a single-statement upsert.
CREATE UNIQUE INDEX vpn_client_mfa_session_location_device_unique
    ON vpn_client_mfa_session (location_id, device_id);
