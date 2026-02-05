ALTER TABLE proxy DROP COLUMN public_address;
ALTER TABLE settings ADD COLUMN public_proxy_url TEXT NOT NULL DEFAULT 'http://localhost:8080';
