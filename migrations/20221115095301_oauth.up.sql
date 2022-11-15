ALTER TABLE oauth2client ADD user_id bigint NULL;
UPDATE oauth2client SET user_id = "user".id FROM "user" WHERE "user".username = oauth2client.user;
ALTER TABLE oauth2client DROP "user";
DELETE FROM oauth2client WHERE user_id IS NULL;
ALTER TABLE oauth2client ALTER user_id SET NOT NULL;
ALTER TABLE oauth2client ADD FOREIGN KEY(user_id) REFERENCES "user"(id);
