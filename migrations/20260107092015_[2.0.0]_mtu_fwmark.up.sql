ALTER TABLE wireguard_network
    ADD COLUMN mtu integer NOT NULL DEFAULT 1420,
    ADD COLUMN fwmark bigint NOT NULL DEFAULT 0;
