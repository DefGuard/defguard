CREATE TYPE connection_direction AS ENUM (
    'inbound',
    'outbound'
);

CREATE TYPE connection_status AS ENUM (
    'allowed',
    'blocked'
);

-- Connection events reported by Gateways.
CREATE TABLE connection_event (
    id bigserial PRIMARY KEY,
    location_id bigint NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    -- No foreign key: events must outlive deleted Gateways.
    gateway_id bigint NOT NULL,
    timestamp timestamp without time zone NOT NULL,
    direction connection_direction NOT NULL,
    status connection_status NOT NULL,
    -- IANA protocol number.
    protocol integer NOT NULL,
    source_ip inet NOT NULL,
    source_port integer NULL,
    destination_ip inet NOT NULL,
    destination_port integer NULL,
    -- No foreign key: ACL rules may be deleted, and events must outlive them.
    rule_id bigint NULL,
    upload_bytes bigint NOT NULL,
    download_bytes bigint NOT NULL,
    count bigint NOT NULL
);

-- Rows arrive roughly in timestamp order, so BRIN is enough for purging old events.
CREATE INDEX connection_event_timestamp_idx ON connection_event USING BRIN (timestamp);
-- Serves ON DELETE CASCADE from wireguard_network and per-location queries.
CREATE INDEX connection_event_location_id_timestamp_idx ON connection_event (location_id, timestamp);
