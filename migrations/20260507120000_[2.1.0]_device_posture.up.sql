CREATE TYPE os_type AS ENUM (
    'windows',
    'macos',
    'linux',
    'ios',
    'android'
);

CREATE TABLE device_posture (
    id                      BIGSERIAL PRIMARY KEY,
    name                    TEXT NOT NULL,
    description             TEXT,
    min_client_version      TEXT,
    allow_prerelease_client BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE device_posture_os_rule (
    id                              BIGSERIAL PRIMARY KEY,
    posture_id                      BIGINT  NOT NULL REFERENCES device_posture(id) ON DELETE CASCADE,
    os_type                         os_type NOT NULL,
    min_os_version                  TEXT,
    disk_encryption_required        BOOLEAN,
    -- windows only
    antivirus_required              BOOLEAN,
    ad_domain_joined_required       BOOLEAN,
    windows_security_update_current BOOLEAN,
    -- linux only
    min_kernel_version              TEXT,
    -- macos, android only
    device_integrity_required       BOOLEAN,
    UNIQUE (posture_id, os_type)
);

CREATE TABLE device_posture_location (
    posture_id  BIGINT NOT NULL REFERENCES device_posture(id)    ON DELETE CASCADE,
    location_id BIGINT NOT NULL REFERENCES wireguard_network(id) ON DELETE CASCADE,
    PRIMARY KEY (posture_id, location_id)
);
