ALTER TABLE session DROP web3_challenge;
DROP TABLE wallet;

CREATE TYPE mfa_method_new AS ENUM (
    'none',
    'one_time_password',
    'webauthn',
    'email'
);
UPDATE "user" SET mfa_method = 'none' WHERE mfa_method = 'web3';
ALTER TABLE "user"
    ALTER COLUMN mfa_method DROP DEFAULT,
    ALTER COLUMN mfa_method TYPE mfa_method_new USING mfa_method::TEXT::mfa_method_new,
    ALTER COLUMN mfa_method SET DEFAULT 'none'::mfa_method_new;
DROP TYPE mfa_method;
ALTER TYPE mfa_method_new RENAME TO mfa_method;
