-- 1. Restore settings columns
ALTER TABLE settings
    ADD COLUMN initial_setup_completed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN initial_setup_step initial_setup_step NOT NULL DEFAULT 'welcome';

-- 2. Copy data back from wizard to settings
UPDATE settings SET
    initial_setup_completed = w.completed,
    initial_setup_step = COALESCE((w.initial_setup_state->>'step')::initial_setup_step, 'welcome')
FROM wizard w WHERE w.is_singleton = TRUE AND settings.id = 1;

-- 3. Reverse proxy modified_by: convert back to bigint FK
-- NOTE: name-to-id conversion is lossy; existing rows will have NULL modified_by.
ALTER TABLE proxy
    ALTER COLUMN modified_by TYPE bigint USING NULL,
    ADD CONSTRAINT proxy_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES "user"(id);

-- 4. Reverse gateway modified_by: same as proxy
ALTER TABLE gateway
    ALTER COLUMN modified_by TYPE bigint USING NULL,
    ADD CONSTRAINT proxy_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES "user"(id);

-- 5. Drop wizard table and enum
DROP TABLE wizard;
DROP TYPE active_wizard;
