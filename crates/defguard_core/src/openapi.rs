use defguard_common::{
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
        handlers::{acl, openid_providers},
        snat::handlers as snat,
    },
    error::WebError,
    handlers::{
        ApiResponse, EditGroupInfo, GroupInfo, PasswordChange, PasswordChangeSelf,
        SESSION_COOKIE_NAME, StartEnrollmentRequest, Username, auth,
        group::{self, BulkAssignToGroupsRequest, Groups},
        user::{self, UserDetails},
        wireguard as device, wireguard as network,
        wireguard::AddDeviceResult,
    },
};

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        // /auth
        auth::authenticate,
        auth::logout,
        // /user
        user::list_users,
        user::get_user,
        user::add_user,
        user::start_enrollment,
        user::start_remote_desktop_configuration,
        user::username_available,
        user::modify_user,
        user::delete_user,
        user::change_self_password,
        user::change_password,
        user::reset_password,
        user::delete_security_key,
        user::me,
        user::delete_authorized_app,
        // /group
        group::bulk_assign_to_groups,
        group::list_groups_info,
        group::list_groups,
        group::get_group,
        group::create_group,
        group::modify_group,
        group::delete_group,
        group::add_group_member,
        group::remove_group_member,
        // /device
        device::add_device,
        device::modify_device,
        device::get_device,
        device::delete_device,
        device::list_devices,
        device::list_user_devices,
        // /network
        network::create_network,
        network::modify_network,
        network::delete_network,
        network::list_networks,
        network::network_details,
        // /network/{location_id}/snat
		snat::list_snat_bindings,
		snat::create_snat_binding,
		snat::modify_snat_binding,
		snat::delete_snat_binding,
		// /openid
		openid_providers::add_openid_provider,
		openid_providers::get_openid_provider,
		openid_providers::delete_openid_provider,
		openid_providers::modify_openid_provider,
		openid_providers::list_openid_providers,
		// /acl/rule
		acl::list_acl_rules,
		acl::create_acl_rule,
		acl::apply_acl_rules,
		acl::get_acl_rule,
		acl::update_acl_rule,
		acl::delete_acl_rule,
		// /acl/alias
		acl::list_acl_aliases,
		acl::create_acl_alias,
		acl::get_acl_alias,
		acl::update_acl_alias,
		acl::delete_acl_alias,
		acl::apply_acl_aliases,
		// /acl/destination
		acl::destination::list_acl_destinations,
		acl::destination::create_acl_destination,
		acl::destination::get_acl_destination,
		acl::destination::update_acl_destination,
		acl::destination::delete_acl_destination,
    ),
    components(
        schemas(
            ApiResponse, UserInfo, UserDetails, UserDevice, Groups, Username,
            StartEnrollmentRequest, PasswordChangeSelf, PasswordChange, AddDevice, AddDeviceResult,
            Device, ModifyDevice, BulkAssignToGroupsRequest, GroupInfo, EditGroupInfo, WebError
        ),
    ),
    tags(
        (name = "user", description = "
### Endpoints for managing users
Available actions:
- list all users
- disable/enable user
- CRUD mechanism for handling users
- operations on security key and authorized app
- change user password.
- start remote desktop configuratiion
- trigger enrollment process
        "),
        (name = "group", description = "
### Endpoints for managing groups
Available actions:
- list all groups
- CRUD mechanism for handling groups
- add or delete a group member
- remove group
- bulk assign users to groups
        "),
        (name = "device", description = "
### Endpoints for managing devices

Available actions:
- list all devices or user devices
- CRUD mechanism for handling devices.
        "),
        (name = "network", description = "
### Endpoints that allow to control your networks.

Available actions:
- list all wireguard networks
- CRUD mechanism for handling devices.
        "),
        (name = "SNAT", description = "
### Endpoints that allow you to control user SNAT bindings for your locations.

Available actions:
- list all SNAT bindings
- create new SNAT binding
- modify SNAT binding
- delete SNAT binding
        "),
        (name = "ACL", description = "Access Control Lists (ACL)"),
        (name = "OpenID", description = "OpenID providers"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
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
