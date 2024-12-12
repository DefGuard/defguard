CREATE TABLE "group_permission" (
    group_id bigint REFERENCES "group"(id) ON DELETE CASCADE,
    admin boolean NOT NULL DEFAULT FALSE,
    primary key (group_id)
);

INSERT INTO "group_permission" (group_id, admin) VALUES ((
    SELECT id FROM "group" WHERE name = 'admin'
), true);
