CREATE TYPE aclrule_state_new AS ENUM (
    'applied',
    'new',
    'modified',
    'deleted',
    'expired'
);
ALTER TABLE aclrule
  ALTER COLUMN state DROP DEFAULT,
  ALTER COLUMN state TYPE aclrule_state_new USING state::TEXT::aclrule_state_new,
  ALTER COLUMN state SET DEFAULT 'new'::aclrule_state_new;
DROP TYPE aclrule_state;
ALTER TYPE aclrule_state_new RENAME TO aclrule_state;

