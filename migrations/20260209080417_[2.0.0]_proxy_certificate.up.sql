ALTER TABLE proxy
    DROP COLUMN has_certificate,
    ADD COLUMN certificate text;
