-- External OpenID login
CREATE TABLE openidprovider (
    id bigserial PRIMARY KEY,
    "name" text NOT NULL,
    "base_url" text NOT NULL,
    "client_id" text NOT NULL,
    "client_secret" text NOT NULL,
    "enabled" boolean NOT NULL DEFAULT FALSE,
    CONSTRAINT openidprovider_name_unique UNIQUE ("name"),
    CONSTRAINT openidprovider_client_id_unique UNIQUE ("client_id"),
    CONSTRAINT openidprovider_client_secret_unique UNIQUE ("client_secret")
);

ALTER TABLE "user" ADD COLUMN "openid_login" BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN openid_create_account BOOLEAN NOT NULL DEFAULT TRUE;

-- Make emails unique
-- Deletes duplicated users based on their email. The first (by id) user with a given email is kept.
DELETE FROM
    "user" u1
        USING "user" u2
WHERE
    u1.id > u2.id
    AND u1.email = u2.email;
ALTER TABLE "user" ADD CONSTRAINT "user_email_key" UNIQUE (email);
