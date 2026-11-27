ALTER TYPE device_verification RENAME TO device_verification_status;

CREATE TYPE device_verification_outcome AS ENUM ('verified', 'blocked', 'cancelled');

CREATE TABLE device_verification (
    id bigserial PRIMARY KEY,
    device_id bigint NOT NULL UNIQUE REFERENCES device(id) ON DELETE CASCADE,
    started_by bigint REFERENCES "user"(id) ON DELETE SET NULL,
    secret bytea NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    expires_at timestamp without time zone NOT NULL,
    outcome device_verification_outcome,
    resolved_at timestamp without time zone
);
