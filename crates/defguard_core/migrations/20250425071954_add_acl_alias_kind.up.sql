CREATE TYPE aclalias_kind AS ENUM (
    'destination',
    'component'
);
-- set kind for existing aliases and then remove the default
ALTER TABLE aclalias ADD COLUMN kind aclalias_kind NOT NULL DEFAULT 'destination';
ALTER TABLE aclalias ALTER COLUMN kind DROP DEFAULT;
