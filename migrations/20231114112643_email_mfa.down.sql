-- add new enum type
CREATE TYPE mfa_method_new AS ENUM (
    'none',
    'one_time_password',
    'webauthn',
    'web3'
    );

-- remove `email` from `user` table values
UPDATE "user" SET mfa_method = 'none' WHERE mfa_method = 'email';

-- update `user` table to use new enum
ALTER TABLE "user"
    ALTER COLUMN mfa_method DROP DEFAULT,
    ALTER COLUMN mfa_method TYPE mfa_method_new USING mfa_method::TEXT::mfa_method_new,
    ALTER COLUMN mfa_method SET DEFAULT 'none'::mfa_method_new;

-- remove old enum
DROP TYPE mfa_method;

-- rename new enum
ALTER TYPE mfa_method_new RENAME TO mfa_method;