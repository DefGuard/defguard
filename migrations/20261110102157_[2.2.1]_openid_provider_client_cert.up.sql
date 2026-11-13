ALTER TABLE openidprovider
    ADD COLUMN client_certificate text NULL,
    ADD COLUMN client_private_key text NULL;
