ALTER TABLE wireguard_network ADD COLUMN gateways inet[] NOT NULL DEFAULT array[]::inet[];
