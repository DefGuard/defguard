ALTER TABLE wireguard_network ALTER address TYPE inet[] USING ARRAY[address];
ALTER TABLE wireguard_network RENAME address TO addresses;
