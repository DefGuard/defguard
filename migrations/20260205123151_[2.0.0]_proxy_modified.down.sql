ALTER TABLE proxy
    DROP CONSTRAINT IF EXISTS proxy_modified_by_fkey,
    DROP COLUMN IF EXISTS modified_by,
    DROP COLUMN IF EXISTS modified_at;
