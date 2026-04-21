ALTER TABLE gateway RENAME COLUMN certificate_serial TO certificate;
ALTER TABLE proxy   RENAME COLUMN certificate_serial TO certificate;

ALTER TABLE gateway
    DROP COLUMN core_client_cert_der,
    DROP COLUMN core_client_cert_key_der,
    DROP COLUMN core_client_cert_expiry;

ALTER TABLE proxy
    DROP COLUMN core_client_cert_der,
    DROP COLUMN core_client_cert_key_der,
    DROP COLUMN core_client_cert_expiry;

-- Restore the full row_change() function.
CREATE OR REPLACE FUNCTION row_change() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(TG_TABLE_NAME || '_change',
        json_build_object('operation', TG_OP, 'old', row_to_json(OLD), 'new', row_to_json(NEW))::text
    );
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
