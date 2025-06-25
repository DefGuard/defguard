ALTER TABLE webauthn DROP CONSTRAINT webauthn_user_id_fkey;
ALTER TABLE webauthn ADD FOREIGN KEY(user_id) REFERENCES "user"(id);

ALTER TABLE session DROP CONSTRAINT session_user_id_fkey;
ALTER TABLE session ADD FOREIGN KEY(user_id) REFERENCES "user"(id);

ALTER TABLE device DROP CONSTRAINT device_user_id_fkey;
ALTER TABLE device ADD FOREIGN KEY(user_id) REFERENCES "user"(id);

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

ALTER TABLE oauth2token DROP CONSTRAINT oauth2token_user_id_fkey;
ALTER TABLE oauth2token DROP user_id;

ALTER TABLE oauth2client ALTER scope TYPE text USING array_to_string(scope, ',');
ALTER TABLE oauth2client ALTER redirect_uri TYPE text USING array_to_string(redirect_uri, ',');

ALTER TABLE oauth2client DROP enabled;
ALTER TABLE oauth2client DROP name;

ALTER TABLE oauth2client ADD "user" text NOT NULL;
