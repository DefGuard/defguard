CREATE TABLE mail_context (
    template TEXT NOT NULL,
    section TEXT NOT NULL,
    language_tag TEXT NOT NULL,
    text TEXT NOT NULL,
    CONSTRAINT template_section_language UNIQUE (template, section, language_tag)
);
INSERT INTO mail_context (template, section, language_tag, text) VALUES
    ('desktop-start', 'title', 'en_US', 'You are receiving this email to configure a new desktop client.'),
    ('desktop-start', 'subtitle', 'en_US', 'Please paste this URL and token in your desktop client:'),
    ('desktop-start', 'label_url', 'en_US', 'URL'),
    ('desktop-start', 'label_token', 'en_US', 'Token'),
    ('desktop-start', 'configure', 'en_US', 'Configure your desktop client'),
    ('desktop-start', 'click', 'en_US', 'Click the button or use link below'),
    ('new-device', 'title', 'en_US', 'A new device has been add to your account:'),
    ('new-device', 'label_device', 'en_US', 'Device name'),
    ('new-device', 'label_pubkey', 'en_US', 'Public key'),
    ('mfa-code', 'title', 'en_US', 'Hello,'),
    ('mfa-code', 'subtitle', 'en_US', 'It seems like you are trying to login to Defguard. Here is the code you need to access your account.'),
    ('mfa-code', 'code_is_valid', 'en_US', 'The code is valid for 1 minute');
