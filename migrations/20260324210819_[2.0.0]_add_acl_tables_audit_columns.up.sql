DO $$
DECLARE
    audit_username text;
BEGIN
    SELECT u.username
    INTO audit_username
    FROM "user" u
    JOIN group_user gu ON gu.user_id = u.id
    JOIN "group" g ON g.id = gu.group_id
    WHERE u.is_active = true
        AND g.is_admin = true
    ORDER BY u.id ASC
    LIMIT 1;

    IF audit_username IS NULL THEN
        RAISE EXCEPTION 'ACL audit-column migration requires at least one active admin user';
    END IF;

    EXECUTE format(
        'ALTER TABLE aclrule
            ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ADD COLUMN modified_by text NOT NULL DEFAULT %L',
        audit_username
    );

    EXECUTE format(
        'ALTER TABLE aclalias
            ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ADD COLUMN modified_by text NOT NULL DEFAULT %L',
        audit_username
    );
END $$;
