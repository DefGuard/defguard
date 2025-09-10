ALTER TABLE authorizedapps DROP name text;
ALTER TABLE "user" DROP mfa_enabled;
ALTER TABLE "user" DROP recovery_codes;
ALTER TABLE wireguard_network ALTER allowed_ips DROP NOT NULL;
ALTER TABLE wireguard_network ALTER allowed_ips TYPE text USING array_to_string(allowed_ips, ',');
ALTER TABLE wireguard_network ALTER address TYPE text USING address::text;
