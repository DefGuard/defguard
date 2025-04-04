CREATE TYPE aclalias_state AS ENUM (
    'applied',
    'modified'
);
ALTER TABLE aclalias ADD COLUMN state aclalias_state NOT NULL DEFAULT 'applied';
ALTER TABLE aclalias ADD COLUMN parent_id bigint;
ALTER TABLE aclalias ADD CONSTRAINT alias_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES aclalias (id);
ALTER TABLE aclalias ADD CONSTRAINT alias_unique_parent_id UNIQUE (parent_id);
