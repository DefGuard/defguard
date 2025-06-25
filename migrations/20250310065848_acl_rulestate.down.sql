ALTER TABLE aclrule DROP CONSTRAINT unique_parent_id;
ALTER TABLE aclrule DROP CONSTRAINT parent_id_fkey;
ALTER TABLE aclrule DROP COLUMN parent_id;
ALTER TABLE aclrule DROP COLUMN state;
DROP TYPE aclrule_state;
