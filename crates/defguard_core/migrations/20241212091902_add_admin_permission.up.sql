ALTER TABLE "group" ADD COLUMN is_admin boolean NOT NULL DEFAULT FALSE;
-- First group created by migrations is the admin group,
-- which until now couldn't be deleted, so we can assume that it should 
-- have the ID of 1.
UPDATE "group" SET is_admin = true WHERE id = 1;
