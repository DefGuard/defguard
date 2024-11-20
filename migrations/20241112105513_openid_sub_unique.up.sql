-- Make openid_sub unique.
-- This migration may fail if duplicate openid_subs exist in the database.
ALTER TABLE "user" ADD CONSTRAINT "user_openid_sub_key" UNIQUE (openid_sub);
ALTER TABLE openidprovider ADD COLUMN display_name TEXT DEFAULT NULL;
