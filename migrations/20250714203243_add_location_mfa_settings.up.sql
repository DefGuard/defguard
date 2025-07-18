-- add enum representing location MFA configuration
CREATE TYPE location_mfa_mode AS ENUM (
    'disabled',
    'internal',
    'external'
);

-- add nullable column to `wireguard_network` table
ALTER TABLE wireguard_network ADD COLUMN "location_mfa_mode" location_mfa_mode DEFAULT 'disabled';

-- populate new column based on value in `mfa_enabled` column
-- previously only internal MFA was available
UPDATE wireguard_network
SET location_mfa_mode = CASE
    WHEN mfa_enabled = true THEN 'internal'::location_mfa_mode
    ELSE 'disabled'::location_mfa_mode
END;

-- make new column NOT NULL
ALTER TABLE wireguard_network ALTER COLUMN "location_mfa_mode" SET NOT NULL;

-- drop the `mfa_enabled` column since it's no longer needed
ALTER TABLE wireguard_network DROP COLUMN mfa_enabled;

-- remove `use_openid_for_mfa` setting
ALTER TABLE settings DROP COLUMN use_openid_for_mfa;
