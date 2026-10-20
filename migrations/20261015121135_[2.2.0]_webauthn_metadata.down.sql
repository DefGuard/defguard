ALTER TABLE webauthn
    DROP COLUMN authenticator_name,
    DROP COLUMN authenticator_kind,
    DROP COLUMN registration_source;

DROP TYPE webauthn_authenticator_kind;
DROP TYPE webauthn_registration_source;
