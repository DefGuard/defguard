CREATE TABLE group_client_traffic_policy (
    group_id bigint PRIMARY KEY REFERENCES "group"(id) ON DELETE CASCADE,
    client_traffic_policy client_traffic_policy NOT NULL
);
