ALTER TABLE gateway ADD COLUMN has_certificate boolean NOT NULL DEFAULT false;
ALTER TABLE gateway ADD COLUMN certificate_expiry timestamp without time zone NULL;
