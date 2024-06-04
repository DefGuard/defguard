ALTER TABLE "user" ADD COLUMN is_active boolean NOT NULL DEFAULT false;

-- Update the user table to keep the old behaviour
-- Previously: active user = user with password set
UPDATE "user" SET is_active = TRUE WHERE password_hash IS NOT NULL;