ALTER TABLE wizard DROP COLUMN auto_adoption_wizard_needed,
    DROP COLUMN auto_adoption_wizard_state,
    DROP COLUMN auto_adoption_wizard_completed,
    DROP COLUMN auto_adoption_wizard_in_progress;

-- NOTE: conversion from name back to id is not possible; existing rows will be set to NULL.
ALTER TABLE proxy
    ALTER COLUMN modified_by TYPE bigint USING NULL,
    ADD CONSTRAINT proxy_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES "user"(id);

ALTER TABLE gateway
    ALTER COLUMN modified_by TYPE bigint USING NULL,
    ADD CONSTRAINT proxy_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES "user"(id);
