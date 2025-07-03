DROP TABLE openidprovider;
ALTER TABLE "user" DROP CONSTRAINT "user_email_key";
ALTER TABLE "user" DROP COLUMN "openid_login";
ALTER TABLE settings DROP COLUMN openid_create_account;
