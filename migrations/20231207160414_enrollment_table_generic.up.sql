ALTER TABLE enrollment ALTER admin_id DROP NOT NULL;
ALTER TABLE enrollment ADD COLUMN token_type text DEFAULT 'ENROLLMENT';
ALTER TABLE enrollment RENAME TO token;
