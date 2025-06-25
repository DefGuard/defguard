ALTER TABLE openidprovider DROP COLUMN display_name;
ALTER TABLE "user" DROP CONSTRAINT "user_openid_sub_key";
