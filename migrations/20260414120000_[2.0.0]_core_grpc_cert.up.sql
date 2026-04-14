ALTER TABLE gateway RENAME COLUMN certificate TO certificate_serial;
ALTER TABLE proxy   RENAME COLUMN certificate TO certificate_serial;

ALTER TABLE gateway
    ADD COLUMN core_client_cert_der bytea DEFAULT NULL,
    ADD COLUMN core_client_cert_key_der bytea DEFAULT NULL,
    ADD COLUMN core_client_cert_expiry timestamp without time zone NULL;

ALTER TABLE proxy
    ADD COLUMN core_client_cert_der bytea DEFAULT NULL,
    ADD COLUMN core_client_cert_key_der bytea DEFAULT NULL,
    ADD COLUMN core_client_cert_expiry timestamp without time zone NULL;
