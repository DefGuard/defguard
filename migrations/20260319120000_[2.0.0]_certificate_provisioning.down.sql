ALTER TABLE settings
    ADD COLUMN IF NOT EXISTS ca_cert_der                 BYTEA,
    ADD COLUMN IF NOT EXISTS ca_key_der                  BYTEA,
    ADD COLUMN IF NOT EXISTS ca_expiry                   TIMESTAMP WITHOUT TIME ZONE;

UPDATE settings s
SET
    ca_cert_der = c.ca_cert_der,
    ca_key_der  = c.ca_key_der,
    ca_expiry   = c.ca_expiry
FROM certificates c
WHERE s.id = 1 AND c.id = 1;

DROP TABLE IF EXISTS certificates;
