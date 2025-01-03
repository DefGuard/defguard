ALTER TABLE device DROP CONSTRAINT name_user;
ALTER TABLE device ADD CONSTRAINT name_user UNIQUE (name, user_id);
