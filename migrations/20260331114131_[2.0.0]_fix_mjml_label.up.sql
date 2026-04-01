DELETE FROM mail_context WHERE template = 'mfa-code' AND section = 'code_is_valid' AND language_tag = 'en_US';
INSERT INTO mail_context (template, section, language_tag, text) VALUES
    ('mfa-code', 'code_is_valid', 'en_US', 'The code is valid for:');
