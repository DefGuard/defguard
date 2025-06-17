use crate::db::{
    models::authentication_key::AuthenticationKeyType, Device, Group, Id, MFAMethod, User, WebHook,
    WireguardNetwork,
};

#[derive(Serialize)]
pub struct MfaLoginMetadata {
    pub mfa_method: MFAMethod,
}

#[derive(Serialize)]
pub struct DeviceAddedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceRemovedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct NetworkDeviceAddedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct NetworkDeviceRemovedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct NetworkDeviceModifiedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct UserAddedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserRemovedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyRemovedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyAddedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct AuditStreamMetadata {
    pub id: Id,
    pub name: String,
}

#[derive(Serialize)]
pub struct VpnClientMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct VpnClientMfaMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
    pub method: MFAMethod,
}

#[derive(Serialize)]
pub struct EnrollmentDeviceAddedMetadata {
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct VpnLocationMetadata {
    pub location: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct ApiTokenMetadata {
    pub owner: User<Id>,
    pub token_name: String,
}

#[derive(Serialize)]
pub struct ApiTokenRenamedMetadata {
    pub owner: User<Id>,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Serialize)]
pub struct OpenIdAppMetadata {
    pub app_id: Id,
    pub app_name: String,
}

#[derive(Serialize)]
pub struct OpenIdAppStateChangedMetadata {
    pub app_id: Id,
    pub app_name: String,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct OpenIdProviderMetadata {
    pub provider_id: Id,
    pub provider_name: String,
}

#[derive(Serialize)]
pub struct GroupsBulkAssignedMetadata {
    pub users: Vec<User<Id>>,
    pub groups: Vec<Group<Id>>,
}

#[derive(Serialize)]
pub struct GroupMetadata {
    pub group: Group<Id>,
}

#[derive(Serialize)]
pub struct GroupAssignedMetadata {
    pub group: Group<Id>,
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct WebHookMetadata {
    pub webhook: WebHook<Id>,
}

#[derive(Serialize)]
pub struct WebHookStateChangedMetadata {
    pub webhook: WebHook<Id>,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct AuthenticationKeyMetadata {
    pub key_id: Id,
    pub key_name: Option<String>,
    pub key_type: AuthenticationKeyType,
}

#[derive(Serialize)]
pub struct AuthenticationKeyRenamedMetadata {
    pub key_id: Id,
    pub key_type: AuthenticationKeyType,
    pub old_name: Option<String>,
    pub new_name: Option<String>,
}
