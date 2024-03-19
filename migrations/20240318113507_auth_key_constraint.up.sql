ALTER TABLE authentication_key
ADD CONSTRAINT user_key_unique UNIQUE (user_id, key_type, key);
