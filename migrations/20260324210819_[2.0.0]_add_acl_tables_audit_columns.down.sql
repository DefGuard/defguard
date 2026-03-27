ALTER TABLE aclrule
    DROP COLUMN modified_by,
    DROP COLUMN modified_at;

ALTER TABLE aclalias
    DROP COLUMN modified_by,
    DROP COLUMN modified_at;
