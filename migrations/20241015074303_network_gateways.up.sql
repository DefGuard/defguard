ALTER TABLE wireguard_network ADD COLUMN gateways text[] NOT NULL DEFAULT array[]::text[];
