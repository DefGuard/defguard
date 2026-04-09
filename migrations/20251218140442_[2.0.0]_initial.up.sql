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
    ADD COLUMN enable_stats_purge boolean NOT NULL DEFAULT true,
    ADD COLUMN stats_purge_frequency_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN stats_purge_threshold_days int4 NOT NULL DEFAULT 30,
    ADD COLUMN enrollment_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN enrollment_send_welcome_email boolean NOT NULL DEFAULT true,
    ADD COLUMN password_reset_token_timeout_hours int4 NOT NULL DEFAULT 24,
    ADD COLUMN enrollment_session_timeout_minutes int4 NOT NULL DEFAULT 10,
    ADD COLUMN password_reset_session_timeout_minutes int4 NOT NULL DEFAULT 10;

ALTER TABLE activity_log_event ALTER COLUMN ip DROP NOT NULL;

ALTER TABLE settings
    ADD CONSTRAINT fk_default_admin
        FOREIGN KEY (default_admin_id) REFERENCES "user"(id)
        ON DELETE SET NULL;

ALTER TABLE wireguard_network
    ADD COLUMN mtu integer NOT NULL DEFAULT 1420,
    ADD COLUMN fwmark bigint NOT NULL DEFAULT 0,
    ADD COLUMN allow_all_groups boolean NOT NULL DEFAULT false;

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

-- Backfill explicit any_* flags so migrated 1.6 aliases keep treating empty
-- destination, port, and protocol inputs as "match any".
WITH alias_destination_input_state AS (
    SELECT
        alias.id,
        COALESCE(cardinality(alias.destination), 0) = 0 AS has_no_destination_addresses,
        NOT EXISTS (
            SELECT 1
            FROM aclaliasdestinationrange AS alias_range
            WHERE alias_range.alias_id = alias.id
        ) AS has_no_destination_ranges,
        COALESCE(cardinality(alias.ports), 0) = 0 AS has_no_ports,
        COALESCE(cardinality(alias.protocols), 0) = 0 AS has_no_protocols
    FROM aclalias AS alias
)
UPDATE aclalias AS alias
SET
    any_address = (
        state.has_no_destination_addresses
        AND state.has_no_destination_ranges
    ),
    any_port = state.has_no_ports,
    any_protocol = state.has_no_protocols
FROM alias_destination_input_state AS state
WHERE state.id = alias.id;

ALTER TABLE aclalias RENAME COLUMN destination TO addresses;

ALTER TABLE aclrule
    ADD COLUMN any_address boolean NOT NULL DEFAULT false,
    ADD COLUMN any_port boolean NOT NULL DEFAULT false,
    ADD COLUMN any_protocol boolean NOT NULL DEFAULT false,
    ADD COLUMN use_manual_destination_settings boolean NOT NULL DEFAULT true,
    ADD COLUMN allow_all_groups boolean NOT NULL DEFAULT false,
    ADD COLUMN deny_all_groups boolean NOT NULL DEFAULT false;

-- Preserve migrated 1.6 rule behavior by separating destination aliases from
-- component aliases: destination aliases define alias-driven destinations,
-- while component aliases only count when they provide concrete inputs.
WITH rule_alias_destination_input_state AS (
    SELECT
        rule_alias.rule_id,
        BOOL_OR(alias.kind = 'destination') AS has_destination_aliases,
        BOOL_OR(alias.kind = 'component' AND NOT alias.any_address) AS has_component_alias_addresses,
        BOOL_OR(alias.kind = 'component' AND NOT alias.any_port) AS has_component_alias_ports,
        BOOL_OR(alias.kind = 'component' AND NOT alias.any_protocol) AS has_component_alias_protocols
    FROM aclrulealias AS rule_alias
    JOIN aclalias AS alias ON alias.id = rule_alias.alias_id
    GROUP BY rule_alias.rule_id
),
-- Rule-local destination inputs must still be checked after alias detection so
-- legacy rules keep manual settings whenever they stored addresses, ranges,
-- ports, or protocols directly on the rule.
rule_destination_input_state AS (
    SELECT
        rule.id,
        COALESCE(cardinality(rule.destination), 0) = 0 AS has_no_destination_addresses,
        NOT EXISTS (
            SELECT 1
            FROM aclruledestinationrange AS rule_range
            WHERE rule_range.rule_id = rule.id
        ) AS has_no_destination_ranges,
        COALESCE(cardinality(rule.ports), 0) = 0 AS has_no_ports,
        COALESCE(cardinality(rule.protocols), 0) = 0 AS has_no_protocols,
        COALESCE(alias_state.has_destination_aliases, false) AS has_destination_aliases,
        NOT COALESCE(alias_state.has_component_alias_addresses, false) AS has_no_component_alias_addresses,
        NOT COALESCE(alias_state.has_component_alias_ports, false) AS has_no_component_alias_ports,
        NOT COALESCE(alias_state.has_component_alias_protocols, false) AS has_no_component_alias_protocols
    FROM aclrule AS rule
    LEFT JOIN rule_alias_destination_input_state AS alias_state ON alias_state.rule_id = rule.id
)
UPDATE aclrule AS rule
SET
    any_address = (
        state.has_no_destination_addresses
        AND state.has_no_destination_ranges
        AND state.has_no_component_alias_addresses
    ),
    any_port = (
        state.has_no_ports
        AND state.has_no_component_alias_ports
    ),
    any_protocol = (
        state.has_no_protocols
        AND state.has_no_component_alias_protocols
    ),
    -- Only switch migrated 1.6 rules away from manual destination settings
    -- when destination aliases were the sole source of destination inputs.
    use_manual_destination_settings = NOT (
        state.has_no_destination_addresses
        AND state.has_no_destination_ranges
        AND state.has_no_ports
        AND state.has_no_protocols
        AND state.has_no_component_alias_addresses
        AND state.has_no_component_alias_ports
        AND state.has_no_component_alias_protocols
        AND state.has_destination_aliases
    ),
    allow_all_groups = false,
    deny_all_groups = false
FROM rule_destination_input_state AS state
WHERE state.id = rule.id;

-- Rename after backfills because these queries must read the legacy 1.6 column
-- names while deriving the new flags and destination-mode settings.
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
    preshared_key text NULL,
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
CREATE UNIQUE INDEX vpn_client_session_active_location_device_unique
    ON vpn_client_session(location_id, device_id)
    WHERE state IN ('new', 'connected');

ALTER TABLE wireguard_network_device
    DROP COLUMN preshared_key,
    DROP COLUMN is_authorized,
    DROP COLUMN authorized_at;

CREATE INDEX wireguard_network_device_network_id_device_id_idx
    ON wireguard_network_device (wireguard_network_id, device_id);

CREATE INDEX device_user_id_device_type_id_idx
    ON device (user_id, device_type, id);

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
CREATE INDEX idx_vpn_session_stats_session_latest_handshake
    ON vpn_session_stats(session_id, latest_handshake DESC);

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
    ('desktop-start', 'title', 'en_US', 'You''re receiving this email to configure a new desktop client'),
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
    ('user-import-blocked', 'notification_text', 'en_US', 'Import of an external user was blocked because it would exceed your current license capacity.'),
    ('mfa-activation', 'title', 'en_US', 'Hello,'),
    ('mfa-activation', 'subtitle', 'en_US', 'You are activating Multi-Factor Authentication using email verification codes.'),
    ('mfa-activation', 'code_is_valid', 'en_US', 'The code is valid for:'),
    ('enrollment-admin-notification', 'title', 'en_US', 'Dear,'),
    ('enrollment-admin-notification', 'message', 'en_US', 'just completed their enrollment process.'),
    ('enrollment-admin-notification', 'goodday', 'en_US', 'Have a good day!'),
    ('gateway-disconnect', 'title', 'en_US', 'Defguard Gateway has just disconnected.'),
    ('gateway-disconnect', 'subtitle', 'en_US', 'Please login to your gateway server and see the logs.'),
    ('gateway-disconnect', 'gateway_label', 'en_US', 'Gateway name:'),
    ('gateway-disconnect', 'ip_address_label', 'en_US', 'Gateway IP address:'),
    ('gateway-disconnect', 'location_label', 'en_US', 'VPN location:'),
    ('gateway-reconnect', 'title', 'en_US', 'Defguard Gateway has just reconnected.'),
    ('gateway-reconnect', 'gateway_label', 'en_US', 'Gateway name:'),
    ('gateway-reconnect', 'ip_address_label', 'en_US', 'Gateway IP address:'),
    ('gateway-reconnect', 'location_label', 'en_US', 'VPN location:'),
    ('mfa-configured', 'title', 'en_US', 'Hello,'),
    ('mfa-configured', 'subtitle', 'en_US', 'A Multi-Factor Authentication (MFA) has been activated in your account.'),
    ('mfa-configured', 'mfa_method_label', 'en_US', 'MFA method:'),
    ('new-device-login', 'title', 'en_US', 'Your account was just logged into from a new device.'),
    ('new-device-login', 'label_device', 'en_US', 'Device name:'),
    ('new-device-login', 'label_date', 'en_US', 'Date:'),
    ('new-device-oidc-login', 'title', 'en_US', 'Your account was just logged into a system using OpenID Connect authorization'),
    ('new-device-oidc-login', 'subtitle', 'en_US', 'You can deauthorize all applications that have access to your account from the web vault under (Profile > Authorized Apps).'),
    ('new-device-oidc-login', 'label_profile', 'en_US', 'Profile URL:'),
    ('new-device-oidc-login', 'label_oauth2client', 'en_US', 'System name:'),
    ('password-reset', 'title', 'en_US', 'Password reset'),
    ('password-reset', 'subtitle', 'en_US', 'If you wish to reset your password, please copy and paste the following URL in your browser:'),
    ('password-reset-done', 'title', 'en_US', 'Password reset'),
    ('password-reset-done', 'subtitle', 'en_US', 'Your password has been successfully changed.'),
    ('test', 'title', 'en_US', 'This is test email from Defguard system.'),
    ('test', 'subtitle', 'en_US', 'If you received it, your SMTP configuration is correct.'),
    ('support-data', 'title', 'en_US', 'Support data'),
    ('support-data', 'subtitle', 'en_US', 'Support data can be found in the attachment.');

CREATE UNIQUE INDEX api_token_token_hash_idx
    ON api_token (token_hash);

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
