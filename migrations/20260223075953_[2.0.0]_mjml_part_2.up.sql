ALTER TABLE mail_context
    ADD COLUMN enabled BOOL NOT NULL DEFAULT true,
    DROP CONSTRAINT template_section_language;
