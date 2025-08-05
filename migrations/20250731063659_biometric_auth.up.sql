CREATE TABLE biometric_auth (
    id bigserial PRIMARY KEY,
    pub_key text NOT NULL,
    device_id bigint NOT NULL,
    FOREIGN KEY(device_id) REFERENCES "device"(id) ON DELETE CASCADE,
    CONSTRAINT biometric_auth_device UNIQUE (device_id)
);
