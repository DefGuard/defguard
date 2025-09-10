DELETE FROM oauth2token;
ALTER TABLE oauth2token DROP user_id;
ALTER TABLE oauth2token DROP oauth2client_id;
CREATE TABLE oauth2authorizedapp (
    id bigserial PRIMARY KEY,
    "oauth2client_id" bigint NOT NULL,
    "user_id" bigint NOT NULL
);
ALTER TABLE oauth2token ADD COLUMN oauth2authorizedapp_id bigint NOT NULL;
ALTER TABLE oauth2token ADD FOREIGN KEY(oauth2authorizedapp_id) REFERENCES "oauth2authorizedapp"(id) ON DELETE CASCADE;
