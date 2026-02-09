ALTER TABLE proxy
    DROP COLUMN has_certificate,
    ADD COLUMN certificate text;

CREATE TABLE revoked_certificates (
    id bigserial PRIMARY KEY,
    certificate text NOT NULL,
    revoked_at timestamp without time zone NOT NULL,
    certificate_expiry timestamp without time zone NOT NULL
);
