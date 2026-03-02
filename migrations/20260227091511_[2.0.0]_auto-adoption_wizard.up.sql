ALTER TABLE wizard ADD COLUMN auto_adoption_wizard_needed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN auto_adoption_wizard_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN auto_adoption_wizard_completed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN auto_adoption_wizard_in_progress BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE proxy ADD COLUMN modified_by_name text;
UPDATE proxy SET modified_by_name = u.first_name || ' ' || u.last_name
    FROM "user" u WHERE u.id = proxy.modified_by;
ALTER TABLE proxy DROP CONSTRAINT proxy_modified_by_fkey, DROP COLUMN modified_by;
ALTER TABLE proxy RENAME COLUMN modified_by_name TO modified_by;
ALTER TABLE proxy ALTER COLUMN modified_by SET NOT NULL;

ALTER TABLE gateway ADD COLUMN modified_by_name text;
UPDATE gateway SET modified_by_name = u.first_name || ' ' || u.last_name
    FROM "user" u WHERE u.id = gateway.modified_by;
ALTER TABLE gateway DROP CONSTRAINT proxy_modified_by_fkey, DROP COLUMN modified_by;
ALTER TABLE gateway RENAME COLUMN modified_by_name TO modified_by;
ALTER TABLE gateway ALTER COLUMN modified_by SET NOT NULL;
