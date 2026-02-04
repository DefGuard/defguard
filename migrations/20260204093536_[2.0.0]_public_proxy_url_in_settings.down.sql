ALTER TABLE proxy ADD COLUMN public_address TEXT NOT NULL;
ALTER TABLE settings DROP COLUMN public_proxy_url;
