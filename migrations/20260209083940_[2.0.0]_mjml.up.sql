CREATE TABLE mail_context (
    template TEXT NOT NULL,
    section TEXT NOT NULL,
    language_tag TEXT NOT NULL,
    text TEXT NOT NULL,
    CONSTRAINT template_section_language UNIQUE (template, section, language_tag)
);
INSERT INTO mail_context (template, section, language_tag, text) VALUES
    ("desktop-start", "header", "en_US", "You're receiving this email to configure a new desktop client."),
    ("desktop-start", "subtitle", "en_US", "Please paste this URL and token in your desktop client:"),
    ("desktop-start", "label_url", "en_US", "URL"),
    ("desktop-start", "label_token", "en_US", "Token"),
    ("desktop-start", "configure", "en_US", "Configure your desktop client"),
    ("desktop-start", "click", "en_US", "Click the button or use link below");
