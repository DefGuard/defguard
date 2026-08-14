-- Drop the durable in-progress MFA session table.
DROP TABLE IF EXISTS vpn_client_mfa_session;

-- Recreate the legacy mfa_method column from mfa_methods[1] (lossy for multi-step).
ALTER TABLE vpn_client_session ADD COLUMN mfa_method vpn_client_mfa_method NULL;
UPDATE vpn_client_session SET mfa_method = mfa_methods[1];
ALTER TABLE vpn_client_session DROP COLUMN mfa_methods;
ALTER TABLE vpn_client_session DROP COLUMN flow_id;
