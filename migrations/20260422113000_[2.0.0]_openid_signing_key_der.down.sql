ALTER TABLE settings
    DROP COLUMN IF EXISTS openid_signing_key_der,
    ADD COLUMN openid_signing_key TEXT;
