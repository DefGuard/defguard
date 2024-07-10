-- TODO(aleksander): Drop duplicate emails before adding the constraint.
ALTER TABLE "user" ADD CONSTRAINT "user_email_key" UNIQUE (email);
