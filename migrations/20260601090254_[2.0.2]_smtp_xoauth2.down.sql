ALTER TABLE settings
  DROP smtp_authentication,
  DROP smtp_oauth_issuer_url,
  DROP smtp_oauth_client_id,
  DROP smtp_oauth_client_secret,
  DROP smtp_oauth_refresh_token;
DROP TYPE smtp_authentication;
