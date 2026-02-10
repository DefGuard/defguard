ALTER TABLE proxy ADD COLUMN has_certificate boolean;
UPDATE proxy SET has_certificate = (certificate IS NOT NULL);
ALTER TABLE proxy DROP COLUMN certificate;
