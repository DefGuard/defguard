DROP TABLE enrollment;

ALTER TABLE "user" ALTER COLUMN password_hash SET NOT NULL;

ALTER TABLE settings DROP COLUMN enrollment_vpn_step_optional;
ALTER TABLE settings DROP COLUMN enrollment_welcome_message;
ALTER TABLE settings DROP COLUMN enrollment_welcome_email;
