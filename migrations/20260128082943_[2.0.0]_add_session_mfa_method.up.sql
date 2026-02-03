-- Add enum for MFA methods
CREATE TYPE vpn_client_mfa_method AS ENUM (
    'totp',
    'email',
    'oidc',
    'biometric',
    'mobileapprove'
);

-- Add MFA method column to VPN client session
ALTER TABLE vpn_client_session ADD COLUMN mfa_method vpn_client_mfa_method NULL;

-- Remove unnecessary MFA type from VPN client session
ALTER TABLE vpn_client_session DROP COLUMN mfa_mode;

