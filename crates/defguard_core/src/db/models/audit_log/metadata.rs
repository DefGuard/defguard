use crate::{
    db::{
        models::{authentication_key::AuthenticationKey, oauth2client::OAuth2Client},
        Device, Group, Id, MFAMethod, User, WebAuthn, WebHook, WireguardNetwork,
    },
    enterprise::db::models::{
        api_tokens::ApiToken, audit_stream::AuditStream, openid_provider::OpenIdProvider,
    },
};

#[derive(Serialize)]
pub struct MfaLoginMetadata {
    pub mfa_method: MFAMethod,
}

#[derive(Serialize)]
pub struct DeviceMetadata {
    pub owner: User<Id>,
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub owner: User<Id>,
    pub before: Device<Id>,
    pub after: Device<Id>,
}

#[derive(Serialize)]
pub struct NetworkDeviceMetadata {
    pub device: Device<Id>,
    pub location: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct NetworkDeviceModifiedMetadata {
    pub location: WireguardNetwork<Id>,
    pub before: Device<Id>,
    pub after: Device<Id>,
}

#[derive(Serialize)]
pub struct UserMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub before: User<Id>,
    pub after: User<Id>,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyMetadata {
    pub key: WebAuthnMetadata,
}

// Avoid storing secrets in metadata
#[derive(Serialize)]
pub struct WebAuthnMetadata {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
}

impl From<WebAuthn<Id>> for WebAuthnMetadata {
    fn from(value: WebAuthn<Id>) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            name: value.name,
        }
    }
}

#[derive(Serialize)]
pub struct AuditStreamMetadata {
    pub stream: AuditStream<Id>,
}

#[derive(Serialize)]
pub struct AuditStreamModifiedMetadata {
    pub before: AuditStream<Id>,
    pub after: AuditStream<Id>,
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
pub struct EnrollmentTokenMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct VpnLocationMetadata {
    pub location: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct VpnLocationModifiedMetadata {
    pub before: WireguardNetwork<Id>,
    pub after: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct ApiTokenMetadata {
    pub owner: User<Id>,
    pub token: ApiToken<Id>,
}

#[derive(Serialize)]
pub struct ApiTokenRenamedMetadata {
    pub owner: User<Id>,
    pub token: ApiToken<Id>,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Serialize)]
pub struct OpenIdAppMetadata {
    pub app: OAuth2Client<Id>,
}

#[derive(Serialize)]
pub struct OpenIdAppModifiedMetadata {
    pub before: OAuth2Client<Id>,
    pub after: OAuth2Client<Id>,
}

#[derive(Serialize)]
pub struct OpenIdAppStateChangedMetadata {
    pub app: OAuth2Client<Id>,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct OpenIdProviderMetadata {
    pub provider: OpenIdProvider<Id>,
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
pub struct GroupModifiedMetadata {
    pub before: Group<Id>,
    pub after: Group<Id>,
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
pub struct WebHookModifiedMetadata {
    pub before: WebHook<Id>,
    pub after: WebHook<Id>,
}

#[derive(Serialize)]
pub struct WebHookStateChangedMetadata {
    pub webhook: WebHook<Id>,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct AuthenticationKeyMetadata {
    pub key: AuthenticationKey<Id>,
}

#[derive(Serialize)]
pub struct AuthenticationKeyRenamedMetadata {
    pub key: AuthenticationKey<Id>,
    pub old_name: Option<String>,
    pub new_name: Option<String>,
}

#[derive(Serialize)]
pub struct PasswordChangedByAdminMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct PasswordResetMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct ClientConfigurationTokenMetadata {
    pub user: User<Id>,
}
