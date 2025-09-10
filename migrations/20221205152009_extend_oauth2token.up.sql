DELETE FROM oauth2token;
ALTER TABLE oauth2token ADD COLUMN oauth2client_id bigint NOT NULL;
ALTER TABLE oauth2token ADD FOREIGN KEY(oauth2client_id) REFERENCES "oauth2client"(id) ON DELETE CASCADE;
