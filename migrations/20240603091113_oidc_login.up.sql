CREATE TABLE oauth2service (
    id bigserial PRIMARY KEY,
    "client_id" text NOT NULL,
    "client_secret" text NOT NULL,
    "auth_url" text NOT NULL
);
