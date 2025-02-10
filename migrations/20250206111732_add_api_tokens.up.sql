CREATE TABLE api_token (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    name text NOT NULL,
    token_hash text NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE
);
