UPDATE mail_context SET text = 'You''re receiving this email to configure a new desktop client'
WHERE template = 'desktop-start' AND section = 'title' AND language_tag = 'en_US';

UPDATE mail_context SET text = 'You can deauthorize all applications that have access to your account from the web vault under (Profile > Authorized Apps).'
WHERE template = 'new-device-oidc-login' AND section = 'subtitle' AND language_tag = 'en_US';
