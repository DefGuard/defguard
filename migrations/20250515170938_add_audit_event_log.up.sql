CREATE TYPE audit_module AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment'
);

CREATE TABLE audit_event (
    id bigserial PRIMARY KEY,
    timestamp timestamp without time zone NOT NULL,
    username text NOT NULL,
    ip inet NOT NULL,
    event text NOT NULL,
    module audit_module NOT NULL,
    device text NOT NULL,
    metadata jsonb NULL,
    FOREIGN KEY(username) REFERENCES "user"(username) ON DELETE CASCADE
);
CREATE INDEX audit_event_timestamp_idx ON audit_event(timestamp); 
CREATE INDEX audit_event_username_idx ON audit_event(username); 
CREATE INDEX audit_event_event_idx ON audit_event(event); 
CREATE INDEX audit_event_module_idx ON audit_event(module); 
