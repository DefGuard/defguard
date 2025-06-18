CREATE EXTENSION IF NOT EXISTS "pgcrypto";
ALTER TABLE settings
ADD uuid UUID DEFAULT gen_random_uuid() NOT NULL;
