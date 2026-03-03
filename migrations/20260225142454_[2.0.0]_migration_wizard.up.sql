-- 1. Create the active_wizard enum type
CREATE TYPE active_wizard AS ENUM ('none', 'initial', 'auto_adoption', 'migration');

-- 2. Create wizard table: active_wizard and completed as columns, each wizard
--    type has its own JSONB column for step tracking state
CREATE TABLE wizard (
    is_singleton BOOLEAN NOT NULL DEFAULT TRUE PRIMARY KEY CHECK (is_singleton),
    active_wizard active_wizard NOT NULL DEFAULT 'none',
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    initial_setup_state JSONB,
    auto_adoption_state JSONB,
    migration_wizard_state JSONB
);

-- 3. Migrate initial_setup data from settings into wizard
INSERT INTO wizard (is_singleton, active_wizard, completed, initial_setup_state)
SELECT TRUE, 'none'::active_wizard, s.initial_setup_completed,
       jsonb_build_object('step', s.initial_setup_step::text)
FROM settings s WHERE s.id = 1;

-- 4. Drop wizard-related columns from settings
ALTER TABLE settings
    DROP COLUMN initial_setup_completed,
    DROP COLUMN initial_setup_step;

-- 5. Proxy modified_by: convert from user id (bigint FK) to user name (text)
ALTER TABLE proxy ADD COLUMN modified_by_name text;
UPDATE proxy SET modified_by_name = u.first_name || ' ' || u.last_name
    FROM "user" u WHERE u.id = proxy.modified_by;
ALTER TABLE proxy DROP CONSTRAINT proxy_modified_by_fkey, DROP COLUMN modified_by;
ALTER TABLE proxy RENAME COLUMN modified_by_name TO modified_by;
ALTER TABLE proxy ALTER COLUMN modified_by SET NOT NULL;

-- 6. Gateway modified_by: convert from user id (bigint FK) to user name (text)
ALTER TABLE gateway ADD COLUMN modified_by_name text;
UPDATE gateway SET modified_by_name = u.first_name || ' ' || u.last_name
    FROM "user" u WHERE u.id = gateway.modified_by;
ALTER TABLE gateway DROP CONSTRAINT proxy_modified_by_fkey, DROP COLUMN modified_by;
ALTER TABLE gateway RENAME COLUMN modified_by_name TO modified_by;
ALTER TABLE gateway ALTER COLUMN modified_by SET NOT NULL;
