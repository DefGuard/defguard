CREATE TYPE os_type AS ENUM (
    'windows',
    'macos',
    'linux',
    'ios',
    'android'
);

CREATE TABLE device_posture (
    id                      bigserial PRIMARY KEY,
    name                    text    NOT NULL,
    description             text,
    min_client_version      text,
    allow_prerelease_client boolean NOT NULL DEFAULT false
);

CREATE TABLE device_posture_os_rule (
    id         bigserial PRIMARY KEY,
    posture_id bigint  NOT NULL REFERENCES device_posture(id) ON DELETE CASCADE,
    os_type    os_type NOT NULL,
    min_os_version                  text,
    -- windows, macos, linux
    disk_encryption_required        boolean,
    -- windows only
    antivirus_required              boolean,
    ad_domain_joined_required       boolean,
    windows_security_update_current boolean,
    -- linux only
    min_kernel_version              text,
    -- macos, android only
    device_integrity_required       boolean,
    UNIQUE (posture_id, os_type)
);

CREATE TABLE device_posture_location (
    posture_id  bigint NOT NULL REFERENCES device_posture(id)    ON DELETE CASCADE,
    location_id bigint NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    PRIMARY KEY (posture_id, location_id)
);
