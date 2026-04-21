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

-- Switch to a lightweight notification payload (id + operation only) to avoid
-- exceeding PostgreSQL's 8000-byte pg_notify limit when bytea cert columns are populated.
CREATE OR REPLACE FUNCTION row_change() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        TG_TABLE_NAME || '_change',
        json_build_object(
            'operation', TG_OP,
            'id', COALESCE(NEW.id, OLD.id)
        )::text
    );
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
