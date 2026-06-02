ALTER TYPE smtp_encryption ADD VALUE IF NOT EXISTS 'xoauth2';
ALTER TABLE settings
  ADD smtp_oauth_issuer_url text NULL,
  ADD smtp_oauth_client_id text NULL,
  ADD smtp_oauth_client_secret text NULL,
  ADD smtp_oauth_refresh_token text NULL;
