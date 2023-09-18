ALTER TABLE settings
DROP COLUMN uuid;
-- Drop the "uuid-ossp" extension
DROP EXTENSION IF EXISTS "uuid-ossp";
