CREATE TABLE user_snat_binding (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL,
    location_id bigint NOT NULL,
    public_ip inet NOT NULL,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY(location_id) REFERENCES "wireguard_network"(id) ON DELETE CASCADE,
    CONSTRAINT user_location UNIQUE (user_id, location_id)
);
