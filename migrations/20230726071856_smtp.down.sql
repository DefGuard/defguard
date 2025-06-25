ALTER TABLE settings DROP COLUMN smtp_server;
ALTER TABLE settings DROP COLUMN smtp_port;
ALTER TABLE settings DROP COLUMN smtp_encryption;
ALTER TABLE settings DROP COLUMN smtp_user;
ALTER TABLE settings DROP COLUMN smtp_password;
ALTER TABLE settings DROP COLUMN smtp_sender;
DROP TYPE smtp_encryption;
