ALTER TABLE wireguard_network
ADD COLUMN allow_all_groups boolean NOT NULL DEFAULT true;
