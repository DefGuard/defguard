CREATE TYPE openid_provider_kind AS ENUM (
    'Custom',
    'Google',
    'Microsoft',
    'Okta',
    'JumpCloud',
    'Zitadel'
);

ALTER TABLE openidprovider ADD COLUMN kind openid_provider_kind NOT NULL DEFAULT 'Custom'::openid_provider_kind;
