CREATE TYPE aclrule_state AS ENUM (
    'applied',
    'new',
    'modified',
    'deleted'
);
ALTER TABLE aclrule ADD COLUMN state aclrule_state NOT NULL DEFAULT 'applied';
ALTER TABLE aclrule ADD COLUMN parent_id bigint;
ALTER TABLE aclrule ADD CONSTRAINT parent_id_fkey FOREIGN KEY (parent_id) REFERENCES aclrule (id);
ALTER TABLE aclrule ADD CONSTRAINT unique_parent_id UNIQUE (parent_id);
