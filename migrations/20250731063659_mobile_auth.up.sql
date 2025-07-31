CREATE TABLE mobile_auth (
    id bigserial PRIMARY KEY,
    pub_key text NOT NULL,
    device_id bigint NOT NULL,
    FOREIGN KEY(device_id) REFERENCES "device"(id) ON DELETE CASCADE,
    CONSTRAINT mobile_auth_device UNIQUE (device_id)
);

CREATE TABLE mobile_challenge (
    id bigserial PRIMARY KEY,
    auth_id bigint,
    challenge text NOT NULL,
    created_at timestamp without time zone NOT NULL,
    FOREIGN KEY(auth_id) REFERENCES "mobile_auth"(id) ON DELETE CASCADE
);
