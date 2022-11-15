ALTER TABLE oauth2client DROP CONSTRAINT oauth2client_user_id_fkey;
ALTER TABLE oauth2client ADD "user" text NULL;
UPDATE oauth2client SET "user" = "user".username FROM "user" WHERE "user".id = oauth2client.user_id;
ALTER TABLE oauth2client ALTER "user" SET NOT NULL;
ALTER TABLE oauth2client DROP user_id;
