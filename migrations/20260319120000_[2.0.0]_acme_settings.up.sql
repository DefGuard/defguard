CREATE TABLE IF NOT EXISTS certificates (
    id                          INTEGER PRIMARY KEY DEFAULT 1,
    -- Core CA
    ca_cert_der                 BYTEA,
    ca_key_der                  BYTEA,
    ca_expiry                   TIMESTAMP WITHOUT TIME ZONE,
    -- Proxy HTTP/HTTPS certificate
    -- Which source is active: 'none' | 'self_signed' | 'letsencrypt' | 'custom'
    proxy_http_cert_source      TEXT NOT NULL DEFAULT 'none',
    proxy_http_cert_pem         TEXT,
    proxy_http_cert_key_pem     TEXT,
    proxy_http_cert_expiry      TIMESTAMP WITHOUT TIME ZONE,
    -- ACME / Let's Encrypt state (only used when source = 'letsencrypt')
    acme_domain                 TEXT,
    acme_account_credentials    TEXT,
    CONSTRAINT single_row CHECK (id = 1)
);

INSERT INTO certificates (id) VALUES (1) ON CONFLICT DO NOTHING;

UPDATE certificates c
SET
    ca_cert_der = s.ca_cert_der,
    ca_key_der  = s.ca_key_der,
    ca_expiry   = s.ca_expiry
FROM settings s
WHERE c.id = 1 AND s.id = 1;

ALTER TABLE settings
    DROP COLUMN IF EXISTS ca_cert_der,
    DROP COLUMN IF EXISTS ca_key_der,
    DROP COLUMN IF EXISTS ca_expiry;
