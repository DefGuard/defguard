ALTER TABLE wireguard_network ALTER address TYPE inet USING address::inet;
ALTER TABLE wireguard_network ALTER allowed_ips TYPE inet[] USING string_to_array(replace(allowed_ips, ' ', ''), ',')::inet[];
ALTER TABLE wireguard_network ALTER allowed_ips SET NOT NULL;
ALTER TABLE "user" ADD recovery_codes text[] NOT NULL DEFAULT array[]::text[];
ALTER TABLE "user" ADD mfa_enabled boolean NOT NULL DEFAULT false; 
ALTER TABLE authorizedapps ADD name text NOT NULL DEFAULT 'app';
