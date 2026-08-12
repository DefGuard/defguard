-- MFA Flow config: entity table
CREATE TABLE mfa_flow (
    id         BIGSERIAL PRIMARY KEY,
    title      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- MFA Flow step: ordered per-flow, methods as PG array
CREATE TABLE mfa_flow_step (
    id       BIGSERIAL PRIMARY KEY,
    flow_id  BIGINT NOT NULL REFERENCES mfa_flow(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    methods  vpn_client_mfa_method[] NOT NULL,
    CONSTRAINT mfa_flow_step_methods_nonempty CHECK (array_length(methods, 1) >= 1),
    CONSTRAINT mfa_flow_step_position_nonneg CHECK (position >= 0)
);
CREATE INDEX idx_mfa_flow_step_flow_id ON mfa_flow_step(flow_id);

-- Location-to-flow assignment with ordered first-match precedence
CREATE TABLE location_mfa_flow (
    location_id BIGINT NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    flow_id     BIGINT NOT NULL REFERENCES mfa_flow(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    is_default  BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (location_id, flow_id)
);

-- Group scoping per assignment
CREATE TABLE location_mfa_flow_group (
    location_id BIGINT NOT NULL,
    flow_id     BIGINT NOT NULL,
    group_id    BIGINT NOT NULL REFERENCES "group"(id) ON DELETE CASCADE,
    PRIMARY KEY (location_id, flow_id, group_id),
    FOREIGN KEY (location_id, flow_id)
        REFERENCES location_mfa_flow(location_id, flow_id) ON DELETE CASCADE
);

-- Stored MFA toggle, independent of assignment presence
ALTER TABLE wireguard_network ADD COLUMN mfa_enabled BOOLEAN NOT NULL DEFAULT false;
UPDATE wireguard_network SET mfa_enabled = (location_mfa_mode <> 'disabled');

-- Backfill: create shared default flows for existing MFA-enabled locations

-- "Default Internal MFA": one step with all internal methods
INSERT INTO mfa_flow (title)
SELECT 'Default Internal MFA'
WHERE EXISTS (SELECT 1 FROM wireguard_network WHERE location_mfa_mode = 'internal');

INSERT INTO mfa_flow_step (flow_id, position, methods)
SELECT mf.id, 0, ARRAY['totp','email','biometric','mobileapprove']::vpn_client_mfa_method[]
FROM mfa_flow mf
WHERE mf.title = 'Default Internal MFA';

INSERT INTO location_mfa_flow (location_id, flow_id, position, is_default)
SELECT wn.id, mf.id, 0, true
FROM wireguard_network wn, mfa_flow mf
WHERE wn.location_mfa_mode = 'internal' AND mf.title = 'Default Internal MFA';

-- "Default External MFA": one step with OIDC
INSERT INTO mfa_flow (title)
SELECT 'Default External MFA'
WHERE EXISTS (SELECT 1 FROM wireguard_network WHERE location_mfa_mode = 'external');

INSERT INTO mfa_flow_step (flow_id, position, methods)
SELECT mf.id, 0, ARRAY['oidc']::vpn_client_mfa_method[]
FROM mfa_flow mf
WHERE mf.title = 'Default External MFA';

INSERT INTO location_mfa_flow (location_id, flow_id, position, is_default)
SELECT wn.id, mf.id, 0, true
FROM wireguard_network wn, mfa_flow mf
WHERE wn.location_mfa_mode = 'external' AND mf.title = 'Default External MFA';

ALTER TABLE wireguard_network DROP COLUMN location_mfa_mode;
