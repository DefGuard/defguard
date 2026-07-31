CREATE TABLE user_directory_identity (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    provider_id bigint NOT NULL REFERENCES openidprovider(id) ON DELETE CASCADE,
    external_id text NOT NULL,
    created timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT user_directory_identity_provider_external_id_unique UNIQUE (provider_id, external_id),
    CONSTRAINT user_directory_identity_user_provider_unique UNIQUE (user_id, provider_id)
);
