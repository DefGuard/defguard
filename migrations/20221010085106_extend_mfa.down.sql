ALTER TABLE webauthn DROP COLUMN name;
ALTER TABLE wallet DROP COLUMN use_for_mfa;
ALTER TABLE "user" DROP COLUMN mfa_method;

DROP TYPE mfa_method;
