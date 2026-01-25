CREATE TABLE proxy (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    address text NOT NULL,
    port integer NOT NULL,
    public_address text NOT NULL,
    connected_at timestamp without time zone NULL,
    disconnected_at timestamp without time zone NULL
);
