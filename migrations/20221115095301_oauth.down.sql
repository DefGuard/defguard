CREATE TABLE openidclientauthcode (
    id bigserial PRIMARY KEY,
    "user" text NOT NULL,
    code text NOT NULL UNIQUE,
    client_id text NOT NULL UNIQUE,
    state text NOT NULL UNIQUE,
    scope text NOT NULL,
    redirect_uri text NOT NULL,
    nonce text
);

ALTER TABLE authorization_code DROP CONSTRAINT authorization_code_user_id_fkey;
ALTER TABLE authorization_code ADD "user" text NULL;
UPDATE authorization_code SET "user" = "user".username FROM "user" WHERE "user".id = authorization_code.user_id;
ALTER TABLE authorization_code ALTER "user" SET NOT NULL;
ALTER TABLE authorization_code DROP user_id;
ALTER TABLE authorization_code DROP code_challenge;
ALTER TABLE authorization_code DROP nonce;

CREATE TABLE openidclient (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    description text NOT NULL,
    home_url text NOT NULL UNIQUE,
    client_id text NOT NULL UNIQUE,
    client_secret text NOT NULL UNIQUE,
    redirect_uri text NOT NULL,
    enabled boolean NOT NULL DEFAULT true
);

ALTER TABLE oauth2client ALTER scope TYPE text USING array_to_string(scope, ',');

ALTER TABLE oauth2client DROP enabled;
ALTER TABLE oauth2client DROP name;

ALTER TABLE oauth2client DROP CONSTRAINT oauth2client_user_id_fkey;
ALTER TABLE oauth2client ADD "user" text NULL;
UPDATE oauth2client SET "user" = "user".username FROM "user" WHERE "user".id = oauth2client.user_id;
ALTER TABLE oauth2client ALTER "user" SET NOT NULL;
ALTER TABLE oauth2client DROP user_id;
