-- Deletes duplicated users based on their email. The first (by id) user with a given email is kept.
DELETE FROM
    "user" u1
        USING "user" u2
WHERE
    u1.id > u2.id
    AND u1.email = u2.email;
ALTER TABLE "user" ADD CONSTRAINT "user_email_key" UNIQUE (email);
