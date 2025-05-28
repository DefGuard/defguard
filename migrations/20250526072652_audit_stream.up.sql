-- Add up migration script here
CREATE TABLE audit_stream (
    id bigserial PRIMARY KEY,
    stream_type text NOT NULL,
    config jsonb NOT NULL,
);