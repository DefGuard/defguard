-- Postgres cannot drop an enum value, so both types are rebuilt and their dependent columns are
-- re-typed through text.

-- Client MFA method. `mfa_flow_step.methods` is the only column of this type: the authorized
-- session no longer stores a method (see the mfa session store migration), and in-progress
-- sessions keep theirs in `vpn_client_mfa_session.steps_snapshot` as JSON, not as this enum.
-- Those snapshots are dropped rather than rewritten - a downgraded server cannot deserialize a
-- FIDO2 method, and the rows are short-lived by construction (`expires_at`).
DELETE FROM vpn_client_mfa_session
WHERE steps_snapshot::text LIKE '%fido2%';

-- A step made up solely of FIDO2 has nothing left once the value is stripped, and an empty
-- `methods` array violates mfa_flow_step_methods_nonempty. Drop those steps before the rewrite.
-- Positions are left as-is: reads are ordered by `position`, so a gap is harmless.
DELETE FROM mfa_flow_step
WHERE methods <@ ARRAY['fido2']::vpn_client_mfa_method[];

UPDATE mfa_flow_step
SET methods = array_remove(methods, 'fido2'::vpn_client_mfa_method)
WHERE 'fido2' = ANY (methods);

CREATE TYPE vpn_client_mfa_method_new AS ENUM (
    'totp',
    'email',
    'oidc',
    'biometric',
    'mobileapprove'
);

ALTER TABLE mfa_flow_step
    ALTER COLUMN methods TYPE vpn_client_mfa_method_new[]
    USING methods::text[]::vpn_client_mfa_method_new[];

DROP TYPE vpn_client_mfa_method;
ALTER TYPE vpn_client_mfa_method_new RENAME TO vpn_client_mfa_method;

-- Web login MFA method.
CREATE TYPE mfa_method_new AS ENUM (
    'none',
    'one_time_password',
    'webauthn',
    'email'
);
UPDATE "user" SET mfa_method = 'none' WHERE mfa_method = 'fido2';
ALTER TABLE "user"
    ALTER COLUMN mfa_method DROP DEFAULT,
    ALTER COLUMN mfa_method TYPE mfa_method_new USING mfa_method::TEXT::mfa_method_new,
    ALTER COLUMN mfa_method SET DEFAULT 'none'::mfa_method_new;
DROP TYPE mfa_method;
ALTER TYPE mfa_method_new RENAME TO mfa_method;
