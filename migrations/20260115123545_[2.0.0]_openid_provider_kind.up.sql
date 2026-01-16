CREATE TYPE openid_provider_kind AS ENUM (
    'Google',
    'Microsoft',
    'Okta',
    'JumpCloud',
    'Custom'
);

ALTER TABLE openidprovider ADD COLUMN kind openid_provider_kind NOT NULL DEFAULT 'Custom'::openid_provider_kind;
