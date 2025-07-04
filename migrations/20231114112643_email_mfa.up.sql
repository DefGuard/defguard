-- add new variant to methods enum
ALTER TYPE mfa_method ADD VALUE 'email';

-- add `email_mfa_enabled` flag and email secret to `user` table
ALTER TABLE "user" ADD COLUMN email_mfa_enabled boolean NOT NULL DEFAULT false;
ALTER TABLE "user" ADD COLUMN email_mfa_secret bytea NULL;
