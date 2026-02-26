CREATE TABLE wizard (
    migration_wizard_needed BOOLEAN NOT NULL DEFAULT FALSE,
    migration_wizard_state JSONB DEFAULT NULL,
    migration_wizard_completed BOOLEAN NOT NULL DEFAULT FALSE,
    migration_wizard_in_progress BOOLEAN NOT NULL DEFAULT FALSE,
    initial_wizard_completed BOOLEAN NOT NULL DEFAULT FALSE,
    initial_wizard_in_progress BOOLEAN NOT NULL DEFAULT FALSE,
    initial_wizard_state JSONB DEFAULT NULL,
    -- Constrain to a single row
    is_singleton BOOLEAN NOT NULL DEFAULT TRUE PRIMARY KEY CHECK (is_singleton)
);
