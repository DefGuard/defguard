-- Drop the durable in-progress MFA session table.
DROP TABLE IF EXISTS vpn_client_mfa_session;

-- Recreate the legacy mfa_method column, left NULL for every row (lossy by construction:
-- which method was used is recorded in the activity log, not recoverable from a boolean).
ALTER TABLE vpn_client_session ADD COLUMN mfa_method vpn_client_mfa_method NULL;
ALTER TABLE vpn_client_session DROP COLUMN is_mfa_session;
