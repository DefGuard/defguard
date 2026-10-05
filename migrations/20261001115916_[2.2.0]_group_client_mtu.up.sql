CREATE TABLE group_client_mtu (
    network_id bigint NOT NULL REFERENCES "wireguard_network"(id) ON DELETE CASCADE,
    group_id bigint NOT NULL REFERENCES "group"(id) ON DELETE CASCADE,
    client_mtu integer NOT NULL,
    PRIMARY KEY (network_id, group_id)
);
