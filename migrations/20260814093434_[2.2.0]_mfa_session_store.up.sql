-- Authorized session records the full ordered method sequence + the governing flow.
ALTER TABLE vpn_client_session ADD COLUMN mfa_methods vpn_client_mfa_method[] NOT NULL DEFAULT '{}';
UPDATE vpn_client_session SET mfa_methods = ARRAY[mfa_method]::vpn_client_mfa_method[]
  WHERE mfa_method IS NOT NULL;
ALTER TABLE vpn_client_session DROP COLUMN mfa_method;

ALTER TABLE vpn_client_session ADD COLUMN flow_id bigint NULL
  REFERENCES mfa_flow(id) ON DELETE SET NULL;
