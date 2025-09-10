-- restore boolean `mfa_enabled` column
ALTER TABLE wireguard_network ADD COLUMN "mfa_enabled" BOOLEAN DEFAULT false;

-- populate based on MFA type
UPDATE wireguard_network
SET mfa_enabled = CASE
    WHEN location_mfa_mode = 'disabled'::location_mfa_mode THEN false
    ELSE true
END;
--
-- make restored column NOT NULL
ALTER TABLE wireguard_network ALTER COLUMN "mfa_enabled" SET NOT NULL;

-- drop new column and type
ALTER TABLE wireguard_network DROP COLUMN "location_mfa_mode";
DROP TYPE location_mfa_mode;

-- restore `use_openid_for_mfa` setting
ALTER TABLE settings ADD COLUMN use_openid_for_mfa BOOLEAN NOT NULL DEFAULT FALSE;
