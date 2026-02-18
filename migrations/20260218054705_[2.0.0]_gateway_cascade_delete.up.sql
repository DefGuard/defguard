ALTER TABLE gateway
DROP CONSTRAINT gateway_network_id_fkey;

ALTER TABLE gateway
ADD CONSTRAINT gateway_network_id_fkey
FOREIGN KEY (network_id)
REFERENCES wireguard_network(id)
ON DELETE CASCADE;
