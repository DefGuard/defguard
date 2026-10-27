CREATE TABLE device_client_info (
    device_id bigint PRIMARY KEY REFERENCES device(id) ON DELETE CASCADE,
    client_version text NULL,
    os_type os_type NOT NULL,
    os_version text NOT NULL,
    os_edition text NULL
);
