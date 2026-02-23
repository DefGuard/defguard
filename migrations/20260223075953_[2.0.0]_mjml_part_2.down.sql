ALTER TABLE mail_context
    DROP COLUMN enabled,
    ADD CONSTRAINT template_section_language UNIQUE (template, section, language_tag);
