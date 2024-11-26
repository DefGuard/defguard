ALTER TABLE wireguard_network RENAME addresses TO address;
ALTER TABLE wireguard_network ALTER address TYPE inet USING address[1];
