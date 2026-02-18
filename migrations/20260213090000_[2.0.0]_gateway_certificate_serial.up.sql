ALTER TABLE gateway
    DROP COLUMN has_certificate,
    ADD COLUMN certificate TEXT;
