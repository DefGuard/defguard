CREATE TYPE smtp_authentication AS ENUM (
    'none',
    'login',
    'xoauth2'
);
ALTER TABLE settings
  ADD smtp_authentication smtp_authentication NOT NULL DEFAULT 'none',
  ADD smtp_oauth_issuer_url text NULL,
  ADD smtp_oauth_client_id text NULL,
  ADD smtp_oauth_client_secret text NULL,
  ADD smtp_oauth_refresh_token text NULL,
  ADD smtp_oauth_tenant_id text NULL;
UPDATE settings SET smtp_authentication = 'login' WHERE smtp_user IS NOT NULL AND smtp_password IS NOT NULL;
