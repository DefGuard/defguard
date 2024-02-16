-- authorization key types
CREATE TYPE authentication_key_type as ENUM (
    'ssh',
    'gpg'
);

CREATE TABLE yubikey (
    id bigserial PRIMARY KEY NOT NULL,
    serial text NOT NULL,
    name text NOT NULL,
    user_id bigint NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE
);

CREATE TABLE authentication_key (
    id bigserial PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    key text NOT NULL,
    key_type authentication_key_type NOT NULL,
    name text NULL,
    created timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    yubikey_id bigint,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY(yubikey_id) REFERENCES "yubikey"(id) ON DELETE CASCADE
);

-- migrate existing keys to new table

-- ssh
INSERT INTO authentication_key (user_id, key, key_type)
SELECT id AS user_id, ssh_key, 'ssh' AS key_type
FROM "user" WHERE ssh_key IS NOT NULL;

-- gpg
INSERT INTO authentication_key (user_id, key, key_type)
SELECT id AS user_id, pgp_key, 'gpg' AS key_type
FROM "user" WHERE pgp_key IS NOT NULL;

-- remove old columns

ALTER TABLE "user" 
DROP COLUMN pgp_key,
DROP COLUMN pgp_cert_id,
DROP COLUMN ssh_key;