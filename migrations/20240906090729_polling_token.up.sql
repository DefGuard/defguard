CREATE TABLE pollingtoken (
    id bigserial PRIMARY KEY,
    token TEXT NOT NULL,
    device_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT now(),
    FOREIGN KEY(device_id) REFERENCES "device"(id) ON DELETE CASCADE
);
