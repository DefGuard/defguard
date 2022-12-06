ALTER TABLE oauth2token ADD client_id bigint NOT NULL;
ALTER TABLE oauth2token ADD FOREIGN KEY(client_id) REFERENCES "oauth2client"(id) ON DELETE CASCADE;
