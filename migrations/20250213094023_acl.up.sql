CREATE TABLE aclrule (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    allow_all_users boolean NOT NULL,
    deny_all_users boolean NOT NULL,
    all_networks boolean NOT NULL,
    destination inet[] NOT NULL, -- TODO: does not solve the "IP range" case
    ports int4range[] NOT NULL,
    protocols int[] NOT NULL,
    expires timestamp without time zone
);

CREATE TABLE aclalias (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    destination inet[] NOT NULL, -- TODO: does not solve the "IP range" case
    ports int4range[] NOT NULL,
    protocols int[] NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT now()
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
