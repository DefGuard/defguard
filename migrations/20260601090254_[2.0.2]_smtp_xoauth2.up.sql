ALTER TABLE settings
  ADD use_xoauth2 boolean NOT NULL DEFAULT false,
  ADD smtp_oauth_issuer_url text NULL,
  ADD smtp_oauth_client_id text NULL,
  ADD smtp_oauth_client_secret text NULL,
  ADD smtp_oauth_refresh_token text NULL;
