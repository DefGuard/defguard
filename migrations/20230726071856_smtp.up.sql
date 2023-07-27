ALTER TABLE settings ADD COLUMN smtp_server text NULL;
ALTER TABLE settings ADD COLUMN smtp_port integer NULL;
ALTER TABLE settings ADD COLUMN smtp_tls boolean NULL;
ALTER TABLE settings ADD COLUMN smtp_user text NULL;
ALTER TABLE settings ADD COLUMN smtp_password text NULL;
ALTER TABLE settings ADD COLUMN smtp_sender text NULL;
