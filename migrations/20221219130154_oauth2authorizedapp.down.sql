DELETE FROM oauth2token;
ALTER TABLE oauth2token DROP oauth2authorizedapp_id;

ALTER TABLE oauth2token ADD COLUMN user_id bigint NOT NULL;
ALTER TABLE oauth2token ADD FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE;

ALTER TABLE oauth2token ADD COLUMN oauth2client_id bigint NOT NULL;
ALTER TABLE oauth2token ADD FOREIGN KEY(oauth2client_id) REFERENCES "oauth2client"(id) ON DELETE CASCADE;

DROP TABLE oauth2authorizedapp;

