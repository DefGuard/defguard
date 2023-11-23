CREATE TABLE device_login_event (
    id bigserial PRIMARY KEY NOT NULL,
    user_id bigint NOT NULL,
    ip_address text NOT NULL,
    model text NULL,
    family text NOT NULL,
    brand text NULL,
    browser text NOT NULL,
    os_family text NOT NULL,
    event_type text NOT NULL,
    created timestamp without time zone NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE
);
