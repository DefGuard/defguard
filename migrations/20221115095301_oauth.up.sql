ALTER TABLE oauth2client ADD user_id bigint NULL;
UPDATE oauth2client SET user_id = "user".id FROM "user" WHERE "user".username = oauth2client.user;
ALTER TABLE oauth2client DROP "user";
DELETE FROM oauth2client WHERE user_id IS NULL;
ALTER TABLE oauth2client ALTER user_id SET NOT NULL;
ALTER TABLE oauth2client ADD FOREIGN KEY(user_id) REFERENCES "user"(id);

ALTER TABLE oauth2client ADD name text NOT NULL DEFAULT 'OAuth2 Application';
ALTER TABLE oauth2client ADD enabled boolean NOT NULL DEFAULT true;

ALTER TABLE oauth2client ALTER scope TYPE text[] USING string_to_array(replace(scope, ' ', ''), ',')::text[];

DROP TABLE openidclient;

ALTER TABLE authorization_code ADD nonce text NULL;
ALTER TABLE authorization_code ADD user_id bigint NULL;
UPDATE authorization_code SET user_id = "user".id FROM "user" WHERE "user".username = authorization_code.user;
ALTER TABLE authorization_code DROP "user";
DELETE FROM authorization_code WHERE user_id IS NULL;
ALTER TABLE authorization_code ALTER user_id SET NOT NULL;
ALTER TABLE authorization_code ADD FOREIGN KEY(user_id) REFERENCES "user"(id);

DROP TABLE openidclientauthcode;
