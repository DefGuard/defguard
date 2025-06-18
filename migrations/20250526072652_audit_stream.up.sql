CREATE TABLE audit_stream (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    stream_type text NOT NULL,
    config jsonb NOT NULL
);
