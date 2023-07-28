DROP TABLE enrollment;

ALTER TABLE "user" ALTER COLUMN password_hash SET NOT NULL;
