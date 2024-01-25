CREATE TABLE authentication_key (
    id bigserial PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    key text NOT NULL,
    key_type text NOT NULL,
    name text NOT NULL,
    created timestamp without time zone NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE
    -- TODO: Hardware key relation
);
