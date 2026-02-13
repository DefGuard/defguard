ALTER TABLE gateway
    DROP COLUMN certificate,
    ADD COLUMN has_certificate boolean NOT NULL DEFAULT false;
