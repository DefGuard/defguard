DROP TABLE authentication_key;

ALTER TABLE "user"
ADD pgp_key text NULL,
ADD pgp_cert_id text NULL,
ADD ssh_key text NULL;
