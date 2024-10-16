CREATE TABLE gateway (
  id bigserial PRIMARY KEY,
  network_id bigint NOT NULL,
  FOREIGN KEY(network_id) REFERENCES wireguard_network(id)
);
