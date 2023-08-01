CREATE TYPE smtp_encryption AS ENUM (
    'none',
    'starttls',
    'implicittls'
);
ALTER TABLE settings ADD COLUMN smtp_server text NULL;
ALTER TABLE settings ADD COLUMN smtp_port integer NULL;
ALTER TABLE settings ADD COLUMN smtp_encryption smtp_encryption NOT NULL default 'starttls';
ALTER TABLE settings ADD COLUMN smtp_user text NULL;
ALTER TABLE settings ADD COLUMN smtp_password text NULL;
ALTER TABLE settings ADD COLUMN smtp_sender text NULL;
