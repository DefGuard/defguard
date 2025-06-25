CREATE UNIQUE INDEX user_key_unique ON authentication_key (user_id, key_type, MD5(key))
