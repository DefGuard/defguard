CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
ALTER TABLE settings
ADD uuid UUID DEFAULT uuid_generate_v4() NOT NULL;
