-- Render the enrollment token/session timeouts dynamically in the "new-account" email.
-- See https://github.com/DefGuard/defguard/issues/3518
UPDATE mail_context
SET text = 'The token is valid for {{ token_timeout }}. Once the enrollment process starts, you have {{ session_timeout }} to complete it.'
WHERE template = 'new-account' AND section = 'token_info' AND language_tag = 'en_US';
