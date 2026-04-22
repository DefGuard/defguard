ALTER TABLE settings
    DROP COLUMN IF EXISTS openid_signing_key,
    ADD COLUMN openid_signing_key_der BYTEA;
