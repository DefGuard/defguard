ALTER TABLE proxy DROP COLUMN has_certificate;
ALTER TABLE proxy DROP COLUMN certificate_expiry;
ALTER TABLE proxy DROP CONSTRAINT unique_address_port;
ALTER TABLE settings DROP COLUMN ca_expiry;
