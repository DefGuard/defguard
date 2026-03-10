-- Settings and network defaults introduced for the 2.0.0 setup flow.
ALTER TABLE settings
    ADD COLUMN ca_key_der bytea DEFAULT NULL,
    ADD COLUMN ca_cert_der bytea DEFAULT NULL,
    ADD COLUMN ca_expiry timestamp without time zone NULL,
    ADD COLUMN defguard_url text NOT NULL DEFAULT 'http://localhost:8000',
    ADD COLUMN default_admin_group_name text NOT NULL DEFAULT 'admin',
    ADD COLUMN authentication_period_days integer NOT NULL DEFAULT 7,
    ADD COLUMN mfa_code_timeout_seconds integer NOT NULL DEFAULT 60,
    ADD COLUMN public_proxy_url text NOT NULL DEFAULT 'http://localhost:8080',
    ADD COLUMN default_admin_id bigint NULL,
    ADD COLUMN secret_key text,
    ADD COLUMN openid_signing_key text,
    ADD COLUMN webauthn_rp_id text,
    ADD COLUMN disable_stats_purge boolean NOT NULL DEFAULT false,
    ADD COLUMN stats_purge_frequency_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN stats_purge_threshold_days int4 NOT NULL DEFAULT 30,
    ADD COLUMN enrollment_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN password_reset_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN enrollment_session_timeout_minutes int4 NOT NULL DEFAULT 10,
    ADD COLUMN password_reset_session_timeout_minutes int4 NOT NULL DEFAULT 10;

ALTER TABLE settings
    ADD CONSTRAINT fk_default_admin
        FOREIGN KEY (default_admin_id) REFERENCES "user"(id)
        ON DELETE SET NULL;

ALTER TABLE wireguard_network
    ADD COLUMN mtu integer NOT NULL DEFAULT 1420,
    ADD COLUMN fwmark bigint NOT NULL DEFAULT 0;

-- External OpenID providers gain a provider kind discriminator.
CREATE TYPE openid_provider_kind AS ENUM (
    'Custom',
    'Google',
    'Microsoft',
    'Okta',
    'JumpCloud',
    'Zitadel'
);

ALTER TABLE openidprovider
    ADD COLUMN kind openid_provider_kind NOT NULL DEFAULT 'Custom'::openid_provider_kind;

-- ACL rules and aliases move to the new "any_*" flags and addresses naming.
ALTER TABLE aclalias
    ADD COLUMN any_address boolean NOT NULL DEFAULT false,
    ADD COLUMN any_port boolean NOT NULL DEFAULT false,
    ADD COLUMN any_protocol boolean NOT NULL DEFAULT false;

UPDATE aclalias
SET
    any_address = array_length(destination, 1) IS NULL,
    any_port = array_length(ports, 1) IS NULL,
    any_protocol = array_length(protocols, 1) IS NULL;

ALTER TABLE aclalias RENAME COLUMN destination TO addresses;

ALTER TABLE aclrule
    ADD COLUMN any_address boolean NOT NULL DEFAULT false,
    ADD COLUMN any_port boolean NOT NULL DEFAULT false,
    ADD COLUMN any_protocol boolean NOT NULL DEFAULT false,
    ADD COLUMN use_manual_destination_settings boolean NOT NULL DEFAULT true,
    ADD COLUMN allow_all_groups boolean NOT NULL DEFAULT false,
    ADD COLUMN deny_all_groups boolean NOT NULL DEFAULT false;

UPDATE aclrule
SET
    any_address = array_length(destination, 1) IS NULL,
    any_port = array_length(ports, 1) IS NULL,
    any_protocol = array_length(protocols, 1) IS NULL;

ALTER TABLE aclrule RENAME COLUMN destination TO addresses;
ALTER TABLE aclrule RENAME COLUMN all_networks TO all_locations;

-- Gateway and proxy management are introduced in their final 2.0.0 form.
CREATE TABLE gateway (
    id bigserial PRIMARY KEY,
    location_id bigint NOT NULL,
    connected_at timestamp without time zone NULL,
    disconnected_at timestamp without time zone NULL,
    certificate_expiry timestamp without time zone NULL,
    version text,
    name text NOT NULL,
    certificate text,
    address text NOT NULL DEFAULT '127.0.0.1',
    port integer NOT NULL DEFAULT 50051,
    modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    enabled boolean NOT NULL DEFAULT true,
    modified_by text NOT NULL,
    CONSTRAINT gateway_network_id_fkey
        FOREIGN KEY (location_id) REFERENCES wireguard_network(id) ON DELETE CASCADE
);

CREATE FUNCTION row_change() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(TG_TABLE_NAME || '_change',
        json_build_object('operation', TG_OP, 'old', row_to_json(OLD), 'new', row_to_json(NEW))::text
    );
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER gateway
    AFTER INSERT OR UPDATE OR DELETE ON gateway
    FOR ROW EXECUTE FUNCTION row_change();

CREATE TABLE proxy (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    address text NOT NULL,
    port integer NOT NULL,
    connected_at timestamp without time zone NULL,
    disconnected_at timestamp without time zone NULL,
    certificate_expiry timestamp without time zone NULL,
    version text,
    modified_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    certificate text,
    modified_by text NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    CONSTRAINT unique_address_port UNIQUE (address, port)
);

-- VPN client session tracking replaces the legacy peer stats model.
CREATE TYPE vpn_client_session_state AS ENUM (
    'new',
    'connected',
    'disconnected'
);

CREATE TYPE vpn_client_mfa_method AS ENUM (
    'totp',
    'email',
    'oidc',
    'biometric',
    'mobileapprove'
);

CREATE TABLE vpn_client_session (
    id bigserial PRIMARY KEY,
    location_id bigint NOT NULL,
    user_id bigint NOT NULL,
    device_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT current_timestamp,
    connected_at timestamp without time zone NULL,
    disconnected_at timestamp without time zone NULL,
    mfa_method vpn_client_mfa_method NULL,
    state vpn_client_session_state NOT NULL DEFAULT 'new',
    FOREIGN KEY (location_id) REFERENCES wireguard_network(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "user"(id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES device(id) ON DELETE CASCADE
);
CREATE INDEX idx_vpn_client_session_user_id ON vpn_client_session(user_id);
CREATE INDEX idx_vpn_client_session_device_id ON vpn_client_session(device_id);
CREATE INDEX idx_vpn_client_session_location_id ON vpn_client_session(location_id);
CREATE INDEX idx_vpn_client_session_state ON vpn_client_session(state);
CREATE INDEX idx_vpn_client_session_created_at ON vpn_client_session(created_at DESC);
CREATE INDEX idx_vpn_client_session_connected_at ON vpn_client_session(connected_at DESC);

CREATE TABLE vpn_session_stats (
    id bigserial PRIMARY KEY,
    session_id bigint NOT NULL,
    gateway_id bigint NOT NULL,
    collected_at timestamp without time zone NOT NULL,
    latest_handshake timestamp without time zone NOT NULL,
    endpoint text NOT NULL,
    total_upload bigint NOT NULL,
    total_download bigint NOT NULL,
    upload_diff bigint NOT NULL,
    download_diff bigint NOT NULL,
    FOREIGN KEY (session_id) REFERENCES vpn_client_session(id) ON DELETE CASCADE,
    FOREIGN KEY (gateway_id) REFERENCES gateway(id) ON DELETE CASCADE
);
CREATE INDEX idx_vpn_session_stats_session_id ON vpn_session_stats(session_id);
CREATE INDEX idx_vpn_session_stats_gateway_id ON vpn_session_stats(gateway_id);
CREATE INDEX idx_vpn_session_stats_collected_at ON vpn_session_stats(collected_at DESC);
CREATE INDEX idx_vpn_session_stats_latest_handshake ON vpn_session_stats(latest_handshake DESC);
CREATE INDEX idx_vpn_session_stats_session_collected ON vpn_session_stats(session_id, collected_at DESC);

-- Remove legacy peer stats structures superseded by VPN session tracking.
DROP VIEW wireguard_peer_stats_view;
DROP TABLE wireguard_peer_stats;

-- Mail template content is moved to the database.
CREATE TABLE mail_context (
    template text NOT NULL,
    section text NOT NULL,
    language_tag text NOT NULL,
    text text NOT NULL,
    enabled bool NOT NULL DEFAULT true
);

INSERT INTO mail_context (template, section, language_tag, text) VALUES
    ('desktop-start', 'title', 'en_US', 'You''re receiving this email to configure a new desktop client.'),
    ('desktop-start', 'subtitle', 'en_US', 'Please paste this URL and token in your desktop client:'),
    ('desktop-start', 'label_url', 'en_US', 'URL'),
    ('desktop-start', 'label_token', 'en_US', 'Token'),
    ('desktop-start', 'configure', 'en_US', 'Configure your desktop client'),
    ('desktop-start', 'click', 'en_US', 'Click the button or use link below'),
    ('new-account', 'title', 'en_US', 'New account has been created for you'),
    ('new-account', 'subtitle', 'en_US', 'To start the enrollment process, please use credentials below.'),
    ('new-account', 'download', 'en_US', 'Download the official Defguard desktop client for your system.'),
    ('new-account', 'after_install', 'en_US', 'After installation, please add a Defguard instance by entering:'),
    ('new-account', 'label_url', 'en_US', 'URL'),
    ('new-account', 'label_token', 'en_US', 'Token'),
    ('new-account', 'token_info', 'en_US', 'The token is valid for 24 hours. Once the enrollment process starts, you have 10 minutes to complete it.'),
    ('new-account', 'label_enroll', 'en_US', 'Enroll with desktop client'),
    ('new-account', 'label_mobile', 'en_US', 'Mobile application'),
    ('new-account', 'scan_qr', 'en_US', 'Scan QR code below to activate Defguard mobile application.'),
    ('new-account', 'mobile_install', 'en_US', 'If you haven''t installed the mobile app, click one of the buttons below.'),
    ('new-account', 'download_google', 'en_US', 'Download from Google Play'),
    ('new-account', 'download_apple', 'en_US', 'Download from Apple Store'),
    ('new-device', 'title', 'en_US', 'A new device has been added to your account:'),
    ('new-device', 'label_device', 'en_US', 'Device name'),
    ('new-device', 'label_pubkey', 'en_US', 'Public key'),
    ('mfa-code', 'title', 'en_US', 'Hello,'),
    ('mfa-code', 'subtitle', 'en_US', 'It seems like you are trying to login to Defguard. Here is the code you need to access your account.'),
    ('mfa-code', 'code_is_valid', 'en_US', 'The code is valid for 1 minute'),
    ('user-import-blocked', 'title', 'en_US', 'User import blocked'),
    ('user-import-blocked', 'notification_text', 'en_US', 'Import of an external user was blocked because it would exceed your current license capacity.');

-- Wizard state is centralized outside of settings.
CREATE TYPE active_wizard AS ENUM ('none', 'initial', 'auto_adoption', 'migration');

CREATE TABLE wizard (
    is_singleton boolean NOT NULL DEFAULT true PRIMARY KEY CHECK (is_singleton),
    active_wizard active_wizard NOT NULL DEFAULT 'none',
    completed boolean NOT NULL DEFAULT false,
    initial_setup_state jsonb,
    auto_adoption_state jsonb,
    migration_wizard_state jsonb
);

INSERT INTO wizard (is_singleton, active_wizard, completed, initial_setup_state)
VALUES (TRUE, 'none'::active_wizard, FALSE, jsonb_build_object('step', 'welcome'));
