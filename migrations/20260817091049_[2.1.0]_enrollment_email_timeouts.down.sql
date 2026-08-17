UPDATE mail_context
SET text = 'The token is valid for 24 hours. Once the enrollment process starts, you have 10 minutes to complete it.'
WHERE template = 'new-account' AND section = 'token_info' AND language_tag = 'en_US';
