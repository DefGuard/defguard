CREATE TABLE enrollment (
    id text PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    admin_id bigint NOT NULL,
    email text NULL,
    created_at timestamp without time zone NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    used_at timestamp without time zone,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY(admin_id) REFERENCES "user"(id)
);

ALTER TABLE "user" ALTER COLUMN password_hash DROP NOT NULL;

ALTER TABLE settings ADD COLUMN enrollment_vpn_step_optional boolean NOT NULL default true;
ALTER TABLE settings ADD COLUMN enrollment_welcome_message text NULL;
ALTER TABLE settings ADD COLUMN enrollment_welcome_email text NULL;
ALTER TABLE settings ADD COLUMN enrollment_use_welcome_message_as_email boolean NOT NULL default true;
