CREATE TYPE audit_module AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment'
);

CREATE TABLE audit_event (
    id bigserial PRIMARY KEY,
    timestamp timestamp without time zone NOT NULL,
    user_id bigint NOT NULL,
    ip inet NOT NULL,
    event text NOT NULL,
    module audit_module NOT NULL,
    device text NOT NULL,
    metadata jsonb NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE
);
