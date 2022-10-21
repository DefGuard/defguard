ALTER TABLE authorizedapps ADD user_id bigint NULL;
UPDATE authorizedapps SET user_id = "user".id FROM "user" WHERE "user".username = authorizedapps.username;
ALTER TABLE authorizedapps DROP username;
ALTER TABLE authorizedapps ALTER user_id SET NOT NULL;
ALTER TABLE authorizedapps ADD FOREIGN KEY(user_id) REFERENCES "user"(id);
