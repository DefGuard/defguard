-- Restore MFA mode column
-- This will not restore a correct MFA mode, but it souldn't be an issue outside of development environments
ALTER TABLE vpn_client_session ADD COLUMN mfa_mode location_mfa_mode NOT NULL DEFAULT 'disabled';

-- Drop MFA method column
ALTER TABLE vpn_client_session DROP COLUMN mfa_method;

-- Drop MFA method enum
DROP TYPE vpn_client_mfa_method;
