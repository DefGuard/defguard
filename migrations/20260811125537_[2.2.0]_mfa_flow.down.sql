DROP TABLE IF EXISTS location_mfa_flow_group;
DROP TABLE IF EXISTS location_mfa_flow;
DROP TABLE IF EXISTS mfa_flow_step;
DROP TABLE IF EXISTS mfa_flow;
ALTER TABLE wireguard_network DROP COLUMN IF EXISTS mfa_enabled;
ALTER TABLE wireguard_network ADD COLUMN IF NOT EXISTS location_mfa_mode location_mfa_mode NOT NULL DEFAULT 'disabled';
