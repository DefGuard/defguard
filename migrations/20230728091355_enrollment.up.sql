CREATE TABLE enrollment (
    id text PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    admin_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    used_at timestamp without time zone,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY(admin_id) REFERENCES "user"(id)
);

ALTER TABLE "user" ALTER COLUMN password_hash DROP NOT NULL;

ALTER TABLE settings ADD COLUMN enrollment_vpn_step_optional boolean NULL;
ALTER TABLE settings ADD COLUMN enrollment_welcome_message text NULL;
ALTER TABLE settings ADD COLUMN enrollment_welcome_email text NULL;
