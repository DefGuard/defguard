DO $$
DECLARE
    audit_username text;
    acl_tables_have_rows boolean;
BEGIN
    LOCK TABLE aclrule, aclalias IN ACCESS EXCLUSIVE MODE;

    SELECT u.username
    INTO audit_username
    FROM "user" u
    JOIN group_user gu ON gu.user_id = u.id
    JOIN "group" g ON g.id = gu.group_id
    WHERE u.is_active = true
        AND g.is_admin = true
    ORDER BY u.id ASC
    LIMIT 1;

    SELECT EXISTS (SELECT 1 FROM aclrule)
        OR EXISTS (SELECT 1 FROM aclalias)
    INTO acl_tables_have_rows;

    IF audit_username IS NULL AND acl_tables_have_rows THEN
        RAISE EXCEPTION 'ACL audit-column migration requires at least one active admin user';
    END IF;

    audit_username := COALESCE(audit_username, 'admin');

    EXECUTE format(
        'ALTER TABLE aclrule
            ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ADD COLUMN modified_by text NOT NULL DEFAULT %L',
        audit_username
    );

    EXECUTE 'ALTER TABLE aclrule ALTER COLUMN modified_by DROP DEFAULT';

    EXECUTE format(
        'ALTER TABLE aclalias
            ADD COLUMN modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ADD COLUMN modified_by text NOT NULL DEFAULT %L',
        audit_username
    );

    EXECUTE 'ALTER TABLE aclalias ALTER COLUMN modified_by DROP DEFAULT';
END $$;
