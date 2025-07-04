ALTER TABLE "user" DROP CONSTRAINT "user_email_key";
CREATE UNIQUE INDEX email_unique_idx on "user" (LOWER(email));
