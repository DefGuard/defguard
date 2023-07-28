CREATE TABLE enrollment (
    id text PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    admin_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    used_at timestamp without time zone,
    FOREIGN KEY(user_id) REFERENCES "user"(id),
    FOREIGN KEY(admin_id) REFERENCES "user"(id)
);

ALTER TABLE "user" ALTER COLUMN password_hash DROP NOT NULL;
