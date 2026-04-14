ALTER TABLE gateway RENAME COLUMN certificate_serial TO certificate;
ALTER TABLE proxy   RENAME COLUMN certificate_serial TO certificate;

ALTER TABLE gateway
    DROP COLUMN IF EXISTS core_client_cert_der,
    DROP COLUMN IF EXISTS core_client_cert_key_der,
    DROP COLUMN IF EXISTS core_client_cert_expiry;

ALTER TABLE proxy
    DROP COLUMN IF EXISTS core_client_cert_der,
    DROP COLUMN IF EXISTS core_client_cert_key_der,
    DROP COLUMN IF EXISTS core_client_cert_expiry;
