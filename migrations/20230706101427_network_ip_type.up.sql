ALTER TABLE wireguard_network_device ALTER COLUMN wireguard_ip type inet USING wireguard_ip::inet;
