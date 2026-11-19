CREATE TYPE openid_provider_kind_new AS ENUM (
    'Custom',
    'Google',
    'Microsoft',
    'Okta',
    'JumpCloud',
    'Zitadel'
);

UPDATE openidprovider SET kind = 'Custom' WHERE kind = 'Keycloak';

ALTER TABLE openidprovider
    ALTER COLUMN kind DROP DEFAULT,
    ALTER COLUMN kind TYPE openid_provider_kind_new USING kind::TEXT::openid_provider_kind_new,
    ALTER COLUMN kind SET DEFAULT 'Custom'::openid_provider_kind_new;

DROP TYPE openid_provider_kind;

ALTER TYPE openid_provider_kind_new RENAME TO openid_provider_kind;
