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
CREATE FUNCTION row_change() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify(TG_TABLE_NAME || '_change',
    json_build_object('operation', TG_OP, 'old', row_to_json(OLD), 'new', row_to_json(NEW))::text
  );
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER gateway
  AFTER INSERT OR UPDATE OR DELETE ON gateway
  FOR ROW EXECUTE FUNCTION row_change();
