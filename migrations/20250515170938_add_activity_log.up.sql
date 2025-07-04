CREATE TYPE activity_log_module AS ENUM (
    'defguard',
    'client',
    'vpn',
    'enrollment'
);

CREATE TABLE activity_log_event (
    id bigserial PRIMARY KEY,
    timestamp timestamp without time zone NOT NULL,
    user_id bigint NOT NULL,
    username text NOT NULL,
    ip inet NOT NULL,
    event text NOT NULL,
    module activity_log_module NOT NULL,
    device text NOT NULL,
    metadata jsonb NULL
);
CREATE INDEX activity_log_event_timestamp_idx ON activity_log_event(timestamp);
CREATE INDEX activity_log_event_username_idx ON activity_log_event(username);
CREATE INDEX activity_log_event_event_idx ON activity_log_event(event);
CREATE INDEX activity_log_event_module_idx ON activity_log_event(module);
