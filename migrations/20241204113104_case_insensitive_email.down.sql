DROP INDEX email_unique_idx;
ALTER TABLE "user" ADD CONSTRAINT "user_email_key" UNIQUE (email);
