CREATE TYPE aclalias_state AS ENUM (
    'applied',
    'modified',
    'deleted'
);
ALTER TABLE aclalias ADD COLUMN state aclalias_state NOT NULL DEFAULT 'applied';
