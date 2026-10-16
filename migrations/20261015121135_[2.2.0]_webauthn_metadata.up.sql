CREATE TYPE webauthn_authenticator_kind AS ENUM ('hardware', 'unknown');
CREATE TYPE webauthn_registration_source AS ENUM ('core', 'desktop', 'mobile');

-- Stable releases only register keys through the web UI, so existing rows are 'core'.
ALTER TABLE webauthn
    ADD COLUMN authenticator_name text NULL,
    ADD COLUMN authenticator_kind webauthn_authenticator_kind NOT NULL DEFAULT 'unknown',
    ADD COLUMN registration_source webauthn_registration_source NOT NULL DEFAULT 'core';
