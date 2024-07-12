CREATE TABLE openidprovider (
    id bigserial PRIMARY KEY,
    "name" text NOT NULL,
    "base_url" text NOT NULL,
    "client_id" text NOT NULL,
    "client_secret" text NOT NULL,
    "enabled" boolean NOT NULL DEFAULT FALSE,
    CONSTRAINT openidprovider_name_unique UNIQUE ("name"),
    CONSTRAINT openidprovider_client_id_unique UNIQUE ("client_id"),
    CONSTRAINT openidprovider_client_secret_unique UNIQUE ("client_secret")
);
