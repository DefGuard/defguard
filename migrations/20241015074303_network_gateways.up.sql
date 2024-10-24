CREATE TABLE gateway (
  id bigserial PRIMARY KEY,
  network_id bigint NOT NULL,
  url text NOT NULL,
  connected boolean NOT NULL DEFAULT false,
  hostname text NULL,
  connected_at timestamp without time zone NULL,
  disconnected_at timestamp without time zone NULL,
  FOREIGN KEY(network_id) REFERENCES wireguard_network(id)
);
