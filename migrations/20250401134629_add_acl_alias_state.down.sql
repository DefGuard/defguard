ALTER TABLE aclalias DROP CONSTRAINT alias_unique_parent_id;
ALTER TABLE aclalias DROP CONSTRAINT alias_parent_id_fkey;
ALTER TABLE aclalias DROP COLUMN parent_id;
ALTER TABLE aclalias DROP COLUMN state;
DROP TYPE aclalias_state;
