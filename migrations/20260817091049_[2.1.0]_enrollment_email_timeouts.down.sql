-- Restore the pre-2.1 default text, but only for rows still carrying the placeholder text, so
-- any text customized after the upgrade is preserved.
UPDATE mail_context
SET text = 'The token is valid for 24 hours. Once the enrollment process starts, you have 10 minutes to complete it.'
WHERE template = 'new-account' AND section = 'token_info' AND language_tag = 'en_US'
  AND text = 'The token is valid for {{ token_timeout }}. Once the enrollment process starts, you have {{ session_timeout }} to complete it.';
