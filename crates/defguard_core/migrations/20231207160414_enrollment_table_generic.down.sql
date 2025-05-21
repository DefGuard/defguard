ALTER TABLE token RENAME TO enrollment;
ALTER TABLE enrollment ALTER admin_id SET NOT NULL;
ALTER TABLE enrollment DROP token_type;
