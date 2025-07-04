ALTER TABLE "user" DROP COLUMN "openid_login";
ALTER TABLE "user" ADD COLUMN "openid_sub" TEXT DEFAULT NULL;
