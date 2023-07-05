CREATE TABLE wireguard_network_allowed_group (
    network_id bigint REFERENCES "wireguard_network"(id) ON DELETE CASCADE,
    group_id bigint REFERENCES "group"(id) ON DELETE CASCADE,
    CONSTRAINT network_group_unique UNIQUE (network_id, group_id)
);
