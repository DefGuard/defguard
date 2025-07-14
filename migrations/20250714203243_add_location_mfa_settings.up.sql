-- add enum representing location MFA configuration
CREATE TYPE location_mfa_type AS ENUM (
    'disabled',
    'internal',
    'external'
);

-- add nullable column to `wireguard_network` table
ALTER TABLE wireguard_network ADD COLUMN "location_mfa" location_mfa_type DEFAULT 'disabled';

-- populate new column based on value in `mfa_enabled` column
-- previously only internal MFA was available
UPDATE wireguard_network
SET location_mfa = CASE
    WHEN mfa_enabled = true THEN 'internal'::location_mfa_type
    ELSE 'disabled'::location_mfa_type
END;

-- make new column NOT NULL
ALTER TABLE wireguard_network ALTER COLUMN "location_mfa" SET NOT NULL;

-- drop the `mfa_enabled` column since it's no longer needed
ALTER TABLE wireguard_network DROP COLUMN mfa_enabled;
