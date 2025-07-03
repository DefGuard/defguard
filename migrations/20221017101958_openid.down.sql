ALTER TABLE authorizedapps DROP CONSTRAINT authorizedapps_user_id_fkey;
ALTER TABLE authorizedapps ADD username text NULL;
UPDATE authorizedapps SET username = "user".username FROM "user" WHERE "user".id = authorizedapps.user_id;
ALTER TABLE authorizedapps ALTER username SET NOT NULL;
ALTER TABLE authorizedapps DROP user_id;
