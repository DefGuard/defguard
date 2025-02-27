CREATE FUNCTION all_ranges_bounded(ranges int4range[]) RETURNS boolean AS $$
BEGIN
    RETURN (
        NOT EXISTS (
            SELECT 1
            FROM unnest(ranges) AS p
            WHERE lower(p) IS NULL OR upper(p) IS NULL
        )
    );
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE TABLE aclrule (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    allow_all_users boolean NOT NULL,
    deny_all_users boolean NOT NULL,
    all_networks boolean NOT NULL,
    destination inet[] NOT NULL,
    ports int4range[] NOT NULL,
    protocols int[] NOT NULL,
    expires timestamp without time zone,
    CONSTRAINT bounded_ports CHECK (all_ranges_bounded(ports))
);

CREATE TABLE aclalias (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    destination inet[] NOT NULL,
    ports int4range[] NOT NULL,
    protocols int[] NOT NULL,
    CONSTRAINT bounded_ports CHECK (all_ranges_bounded(ports))
);

CREATE TABLE aclrulealias (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    alias_id bigint NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    FOREIGN KEY(alias_id) REFERENCES "aclalias"(id) ON DELETE CASCADE,
    CONSTRAINT rule_alias UNIQUE (rule_id, alias_id)
);

CREATE TABLE aclrulenetwork (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    network_id bigint NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    FOREIGN KEY(network_id) REFERENCES "wireguard_network"(id) ON DELETE CASCADE,
    CONSTRAINT rule_network UNIQUE (rule_id, network_id)
);

CREATE TABLE aclruleuser (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    user_id bigint NOT NULL,
    allow bool NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    CONSTRAINT rule_user UNIQUE (rule_id, user_id)
);

CREATE TABLE aclrulegroup (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    group_id bigint NOT NULL,
    allow bool NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    FOREIGN KEY(group_id) REFERENCES "group"(id) ON DELETE CASCADE,
    CONSTRAINT rule_group UNIQUE (rule_id, group_id)
);

CREATE TABLE aclruledevice (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    device_id bigint NOT NULL,
    allow bool NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    FOREIGN KEY(device_id) REFERENCES "device"(id) ON DELETE CASCADE,
    CONSTRAINT rule_device UNIQUE (rule_id, device_id)
);

CREATE TABLE aclruledestinationrange (
    id bigserial PRIMARY KEY,
    rule_id bigint NOT NULL,
    "start" inet NOT NULL,
    "end" inet NOT NULL,
    FOREIGN KEY(rule_id) REFERENCES "aclrule"(id) ON DELETE CASCADE,
    CONSTRAINT no_networks CHECK (host("start")::inet = "start" AND host("end")::inet = "end"),
    CONSTRAINT range_order CHECK ("start" < "end")
);

CREATE TABLE aclaliasdestinationrange (
    id bigserial PRIMARY KEY,
    alias_id bigint NOT NULL,
    "start" inet NOT NULL,
    "end" inet NOT NULL,
    FOREIGN KEY(alias_id) REFERENCES "aclalias"(id) ON DELETE CASCADE,
    CONSTRAINT no_networks CHECK (host("start")::inet = "start" AND host("end")::inet = "end"),
    CONSTRAINT range_order CHECK ("start" < "end")
);
