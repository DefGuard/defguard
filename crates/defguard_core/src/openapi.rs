use defguard_common::{
    CARGO_VERSION,
    db::models::{
        Device,
        device::{AddDevice, ModifyDevice, UserDevice},
    },
    types::user_info::UserInfo,
};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use super::{
    enterprise::{
        handlers::{
            self as enterprise_handlers, acl, activity_log_stream, api_tokens, device_posture,
            enterprise_settings, openid_login, openid_providers,
        },
        snat::handlers as snat,
    },
    handlers::{
        ApiErrorResponse, Auth, EditGroupInfo, GroupInfo, PasswordChange, PasswordChangeSelf,
        SESSION_COOKIE_NAME, StartEnrollmentRequest, Username, activity_log, app_info, auth,
        component_setup, core_certs, forward_auth, gateway,
        group::{self, BulkAssignToGroupsRequest},
        license, location_stats, mail, mfa_flow, network_devices, openid_clients, openid_flow,
        proxy, reserved, resource_display, session_info, settings, ssh_authorized_keys, static_ips,
        support, updates,
        user::{self, UserDetails},
        webhooks, wireguard as device, wireguard as network,
        wireguard::AddDeviceResult,
    },
};

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "defguard Core API",
        version = CARGO_VERSION,
        description = "
REST API of defguard Core.

Authentication is done either with the session cookie obtained from `POST /api/v1/auth`,
or with an API token passed as `Authorization: Bearer <token>`.

API tokens are created by an admin with `POST /api/v1/user/{username}/api_token` and their value is returned only once, in the response to that request.

Errors are returned as a JSON object with a `msg` field and, for some of them, a machine-readable `code` field.",
    ),
    paths(
        // system
        forward_auth::forward_auth,
        crate::health_check,
        app_info::get_app_info,
        updates::outdated_components,
        reserved::check_reserved,
        session_info::get_session_info,
        updates::check_new_version,
        // auth
        auth::authenticate,
        auth::request_email_mfa_code,
        auth::email_mfa_enable,
        auth::email_mfa_init,
        auth::email_mfa_code,
        auth::logout,
        auth::mfa_disable,
        auth::mfa_enable,
        auth::recovery_code,
        auth::totp_enable,
        auth::totp_secret,
        auth::totp_code,
        auth::webauthn_end,
        auth::webauthn_finish,
        auth::webauthn_init,
        auth::webauthn_start,
        // user
        user::me,
        user::list_users,
        user::add_user,
        user::username_available,
        user::bulk_delete_users,
        user::bulk_disable_users,
        user::bulk_enable_users,
        user::bulk_start_enrollment,
        user::change_self_password,
        user::delete_user,
        user::get_user,
        user::modify_user,
        auth::email_mfa_disable,
        auth::disable_user_mfa,
        user::delete_authorized_app,
        user::change_password,
        user::reset_password,
        user::delete_security_key,
        user::start_remote_desktop_configuration,
        user::start_enrollment,
        auth::totp_disable,
        // group
        group::list_groups,
        group::create_group,
        group::list_groups_info,
        group::delete_group,
        group::get_group,
        group::add_group_member,
        group::modify_group,
        group::remove_group_member,
        group::bulk_assign_to_groups,
        // device
        device::list_devices,
        device::list_user_devices,
        device::delete_device,
        device::get_device,
        device::add_device,
        device::modify_device,
        device::user_device_configs,
        // network device
        network_devices::list_network_devices,
        network_devices::add_network_device,
        network_devices::find_available_ips,
        network_devices::check_ip_availability,
        network_devices::start_network_device_setup,
        network_devices::start_network_device_setup_for_device,
        network_devices::get_network_device,
        network_devices::modify_network_device,
        network_devices::network_device_configs,
        // static IP
        static_ips::get_all_user_device_ips,
        static_ips::assign_static_ips,
        static_ips::validate_ip_assignment,
        static_ips::get_device_ips,
        // network
        network::list_networks,
        network::create_network,
        network::count_networks,
        resource_display::get_locations_display,
        network::import_network,
        network::delete_network,
        network::network_details,
        network::modify_network,
        network::download_config,
        network::add_user_devices,
        // location stats
        location_stats::locations_overview_stats,
        location_stats::location_connected_network_devices,
        location_stats::location_connected_users,
        location_stats::location_connected_user_devices,
        location_stats::location_stats,
        // gateway
        gateway::gateway_list,
        gateway::delete_gateway,
        gateway::gateway_details,
        gateway::update_gateway,
        network::all_gateways_status,
        network::gateway_status,
        component_setup::adopt_gateway,
        component_setup::setup_gateway_tls_stream,
        // proxy
        proxy::proxy_list,
        component_setup::stream_proxy_acme,
        component_setup::setup_proxy_tls_stream,
        proxy::delete_proxy,
        proxy::proxy_details,
        proxy::update_proxy,
        // certificates
        core_certs::get_ca,
        core_certs::get_certs,
        core_certs::set_internal_url_settings,
        core_certs::set_external_url_settings,
        // SSH key
        ssh_authorized_keys::get_authorized_keys,
        ssh_authorized_keys::fetch_authentication_keys,
        ssh_authorized_keys::add_authentication_key,
        ssh_authorized_keys::delete_authentication_key,
        ssh_authorized_keys::rename_authentication_key,
        // API token
        api_tokens::fetch_api_tokens,
        api_tokens::add_api_token,
        api_tokens::delete_api_token,
        api_tokens::rename_api_token,
        // webhook
        webhooks::list_webhooks,
        webhooks::add_webhook,
        webhooks::delete_webhook,
        webhooks::get_webhook,
        webhooks::change_enabled,
        webhooks::change_webhook,
        // settings
        settings::get_settings,
        settings::patch_settings,
        settings::update_settings,
        settings::set_default_branding,
        enterprise_settings::get_enterprise_settings,
        enterprise_settings::patch_enterprise_settings,
        settings::get_settings_essentials,
        // LDAP
        settings::ldap_dry_run,
        settings::test_ldap_settings,
        settings::test_submitted_ldap_settings,
        // activity log
        activity_log::get_activity_log_events,
        activity_log_stream::get_activity_log_stream,
        activity_log_stream::create_activity_log_stream,
        activity_log_stream::delete_activity_log_stream,
        activity_log_stream::modify_activity_log_stream,
        // ACL
        acl::alias::list_acl_aliases,
        acl::alias::create_acl_alias,
        acl::alias::apply_acl_aliases,
        acl::alias::count_acl_aliases,
        acl::alias::delete_acl_alias,
        acl::alias::get_acl_alias,
        acl::alias::update_acl_alias,
        acl::destination::list_acl_destinations,
        acl::destination::create_acl_destination,
        acl::destination::apply_acl_destinations,
        acl::destination::count_acl_destinations,
        acl::destination::delete_acl_destination,
        acl::destination::get_acl_destination,
        acl::destination::update_acl_destination,
        acl::list_acl_rules,
        acl::create_acl_rule,
        acl::apply_acl_rules,
        acl::count_acl_rules,
        acl::delete_acl_rule,
        acl::get_acl_rule,
        acl::update_acl_rule,
        // DevicePosture
        device_posture::list_device_postures,
        device_posture::create_device_posture,
        device_posture::get_device_posture_versions,
        device_posture::delete_device_posture,
        device_posture::get_device_posture,
        device_posture::update_device_posture,
        device_posture::duplicate_device_posture,
        device_posture::set_locations_for_posture,
        device_posture::set_postures_for_location,
        // SNAT
        snat::list_snat_bindings,
        snat::create_snat_binding,
        snat::delete_snat_binding,
        snat::modify_snat_binding,
        // OpenID
        openid_login::get_auth_info,
        openid_login::auth_callback,
        openid_providers::list_openid_providers,
        openid_providers::add_openid_provider,
        openid_providers::get_current_openid_provider,
        openid_providers::delete_openid_provider,
        openid_providers::get_openid_provider,
        openid_providers::modify_openid_provider,
        openid_providers::test_dirsync_connection,
        // OAuth2
        openid_flow::openid_configuration,
        openid_clients::list_openid_clients,
        openid_clients::add_openid_client,
        openid_flow::authorization,
        openid_flow::secure_authorization,
        openid_flow::discovery_keys,
        openid_flow::token,
        openid_flow::userinfo,
        openid_clients::delete_openid_client,
        openid_clients::get_openid_client,
        openid_clients::change_openid_client_state,
        openid_clients::change_openid_client,
        // mfa flow
        mfa_flow::list_mfa_flows,
        mfa_flow::create_mfa_flow,
        mfa_flow::get_mfa_flow,
        mfa_flow::update_mfa_flow,
        mfa_flow::delete_mfa_flow,
        mfa_flow::get_location_mfa_flows,
        mfa_flow::set_location_mfa_flows,
        mfa_flow::get_method_availability,
        // support
        mail::send_support_data,
        mail::test_mail,
        support::configuration,
        support::logs,
        // license
        enterprise_handlers::check_enterprise_info,
        license::license_check,
    ),
    components(
        schemas(
            ApiErrorResponse, Auth, UserInfo, UserDetails, UserDevice, Username,
            StartEnrollmentRequest, PasswordChangeSelf, PasswordChange, AddDevice, AddDeviceResult,
            Device, ModifyDevice, BulkAssignToGroupsRequest, GroupInfo, EditGroupInfo,
            license::CheckParams,
            user::SortKey, user::SortOrder, reserved::CheckResource
        ),
    ),
    tags(
        (name = "system", description = "Health check, instance info and other utility endpoints."),
        (name = "auth", description = "User authentication, second authentication factor and session management."),
        (name = "user", description = "User accounts, their MFA methods, enrollment and bulk operations."),
        (name = "group", description = "User groups and their members."),
        (name = "device", description = "User devices and their WireGuard configuration."),
        (name = "network device", description = "Network devices, as opposed to user devices, and their IP addresses."),
        (name = "static IP", description = "Static IP addresses assigned to user devices."),
        (name = "network", description = "WireGuard networks, including import from a WireGuard configuration file."),
        (name = "location stats", description = "Traffic statistics and active connections per location."),
        (name = "gateway", description = "Gateways registered in your locations, their connection status and setup."),
        (name = "proxy", description = "Edge (proxy) instances, their TLS and ACME setup."),
        (name = "certificates", description = "Internal (core) and external (edge) URL certificates."),
        (name = "SSH key", description = "SSH and GPG authentication keys of users."),
        (name = "API token", description = "API tokens used for `Authorization: Bearer` authentication."),
        (name = "webhook", description = "Webhooks triggered by user and provisioning events."),
        (name = "settings", description = "Instance and enterprise settings."),
        (name = "LDAP", description = "LDAP connection tests and sync dry runs."),
        (name = "activity log", description = "Activity log events and activity log streams."),
        (name = "ACL", description = "Access Control Lists (ACL)."),
        (name = "device posture", description = "Device posture check policies."),
        (name = "SNAT", description = "User SNAT bindings in your locations."),
        (name = "OpenID", description = "External OpenID providers used for logging in to defguard."),
        (name = "OAuth2", description = "defguard acting as an OAuth2 / OpenID Connect provider for other applications."),
        (name = "support", description = "Diagnostics, logs and support data."),
        (name = "mfa flow", description = "MFA flow templates — ordered multi-step authentication recipes."),
        (name = "license", description = "Enterprise license."),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.info.license = None;
        if let Some(components) = openapi.components.as_mut() {
            // session cookie auth
            components.add_security_scheme(
                "cookie",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE_NAME))),
            );
            // API token auth
            components.add_security_scheme(
                "api_token",
                SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
            );
        }
    }
}
