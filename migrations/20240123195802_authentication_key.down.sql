DROP TABLE authentication_key;

-- Restore old columns

ALTER TABLE "user"
ADD pgp_key text NULL,
ADD pgp_cert_id text NULL,
ADD ssh_key text NULL;