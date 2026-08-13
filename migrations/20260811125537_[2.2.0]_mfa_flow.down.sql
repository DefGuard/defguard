-- Recreate the legacy enum type (dropped in the up migration). Safe to re-run
-- because the type may still exist when testing against a pre-A2 database.
DROP TYPE IF EXISTS location_mfa_mode;
CREATE TYPE location_mfa_mode AS ENUM ('disabled', 'internal', 'external');

-- Add the legacy column. Nullable because the next steps populate it.
ALTER TABLE wireguard_network ADD COLUMN location_mfa_mode location_mfa_mode;

-- Disabled is the safe default: mfa_enabled=false or no flows assigned.
UPDATE wireguard_network
SET location_mfa_mode = 'disabled'
WHERE NOT mfa_enabled
   OR id NOT IN (SELECT location_id FROM location_mfa_flow);

-- Derive a best-effort legacy mode from the existing flow configuration,
-- using the same shape logic as MfaFlow::derive_legacy_mode.
--
-- A location is legacy-derivable when exactly one flow is assigned and that
-- flow has exactly one step whose method set is the full internal set
-- ({totp, email, biometric, mobileapprove}) or exactly {oidc}.

-- Internal: single-flow, single-step, full internal method set.
UPDATE wireguard_network wn
SET location_mfa_mode = 'internal'
WHERE wn.mfa_enabled
  AND wn.location_mfa_mode IS NULL
  AND (SELECT COUNT(*) FROM location_mfa_flow WHERE location_id = wn.id) = 1
  AND (SELECT COUNT(*) FROM location_mfa_flow lmf
       JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id
       WHERE lmf.location_id = wn.id) = 1
  AND EXISTS (
      SELECT 1 FROM location_mfa_flow lmf
      JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id
      WHERE lmf.location_id = wn.id
      AND mfs.methods @> ARRAY['totp','email','biometric','mobileapprove']::vpn_client_mfa_method[]
      AND ARRAY['totp','email','biometric','mobileapprove']::vpn_client_mfa_method[] @> mfs.methods
  );

-- External: single-flow, single-step, OIDC only.
UPDATE wireguard_network wn
SET location_mfa_mode = 'external'
WHERE wn.mfa_enabled
  AND wn.location_mfa_mode IS NULL
  AND (SELECT COUNT(*) FROM location_mfa_flow WHERE location_id = wn.id) = 1
  AND (SELECT COUNT(*) FROM location_mfa_flow lmf
       JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id
       WHERE lmf.location_id = wn.id) = 1
  AND EXISTS (
      SELECT 1 FROM location_mfa_flow lmf
      JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id
      WHERE lmf.location_id = wn.id
      AND mfs.methods @> ARRAY['oidc']::vpn_client_mfa_method[]
      AND ARRAY['oidc']::vpn_client_mfa_method[] @> mfs.methods
  );

-- Remaining mfa_enabled=true locations have no legacy-equivalent shape
-- (multi-flow, multi-step, or subset-of-internal-methods).
-- Best-effort fallback: internal. This loses fidelity for multi-step and
-- multi-flow configurations, which collapse to the single internal mode.
-- The same is true for subset-of-internal-methods configurations whose
-- available method set is narrower than the full internal set. The
-- overriding goal is to never leave an MFA-enforcing location as disabled.
UPDATE wireguard_network
SET location_mfa_mode = 'internal'
WHERE mfa_enabled AND location_mfa_mode IS NULL;

-- All rows are now populated.
ALTER TABLE wireguard_network ALTER COLUMN location_mfa_mode SET NOT NULL;

-- Drop the flow tables and mfa_enabled, which are no longer needed after the
-- repopulation above.
DROP TABLE IF EXISTS location_mfa_flow_group;
DROP TABLE IF EXISTS location_mfa_flow;
DROP TABLE IF EXISTS mfa_flow_step;
DROP TABLE IF EXISTS mfa_flow;
ALTER TABLE wireguard_network DROP COLUMN IF EXISTS mfa_enabled;
