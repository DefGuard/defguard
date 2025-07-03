CREATE TYPE mfa_method AS ENUM (
    'none',
    'one_time_password',
    'webauthn',
    'web3'
);

ALTER TABLE "user" ADD COLUMN mfa_method mfa_method NOT NULL DEFAULT 'none';
ALTER TABLE wallet ADD COLUMN use_for_mfa boolean NOT NULL DEFAULT true;
ALTER TABLE webauthn ADD COLUMN name text NOT NULL;
