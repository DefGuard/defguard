DROP TABLE authentication_key;

DROP TABLE yubikey;

DROP TYPE authentication_key_type;

ALTER TABLE "user"
ADD pgp_key text NULL,
ADD pgp_cert_id text NULL,
ADD ssh_key text NULL;
