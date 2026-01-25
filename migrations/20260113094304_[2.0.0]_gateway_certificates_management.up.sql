ALTER TABLE gateway
    ADD COLUMN has_certificate boolean NOT NULL DEFAULT false,
    ADD COLUMN certificate_expiry timestamp without time zone NULL;
