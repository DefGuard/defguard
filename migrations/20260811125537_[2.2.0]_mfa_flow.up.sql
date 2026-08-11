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
