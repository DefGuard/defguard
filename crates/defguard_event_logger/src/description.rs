//! Event description generation for activity log.
//!
//! This module provides functions to generate human-readable descriptions for various
//! types of events that occur within the system. These descriptions are used to provide usable
//! context about what happened during each event.
//!
//! Each event type has its own description generator function that takes the event data
//! and returns an optional description string. Some events may not require additional
//! description beyond their event type name, in which case `None` is returned.

use crate::message::{ClientEvent, DefguardEvent, EnrollmentEvent, VpnEvent};

impl DefguardEvent {
    #[must_use]
    pub fn description(&self) -> Option<String> {
        match self {
            Self::UserLoginFailed { message } => Some(format!("User login failed with: {message}")),
            Self::UserMfaLogin { mfa_method } => Some(format!("User logged in using {mfa_method}")),
            Self::UserMfaLoginFailed {
                mfa_method,
                message,
            } => Some(format!(
                "User login using {mfa_method} failed with: {message}"
            )),
            Self::RecoveryCodeLoginFailed => {
                Some("User login with recovery code failed".to_owned())
            }
            Self::UserLogin | Self::UserLogout | Self::RecoveryCodeUsed | Self::PasswordChanged => {
                None
            }
            Self::MfaDisabled => Some("Disabled own MFA".to_owned()),
            Self::UserMfaDisabled { user } => Some(format!("Disabled MFA for user {user}")),
            Self::MfaTotpEnabled => Some("User configured TOTP for MFA".to_owned()),
            Self::MfaTotpDisabled => Some("User disabled TOTP for MFA".to_owned()),
            Self::MfaEmailEnabled => Some("User configured email for MFA".to_owned()),
            Self::MfaEmailDisabled => Some("User disabled email for MFA".to_owned()),
            Self::PasswordChangedByAdmin { user } => {
                Some(format!("Password for user {user} was changed by an admin"))
            }
            Self::PasswordReset { user } => Some(format!("Password for user {user} was reset")),
            Self::MfaSecurityKeyAdded { key } => {
                Some(format!("Added MFA security key {}", key.name))
            }
            Self::MfaSecurityKeyRemoved { key } => {
                Some(format!("Removed MFA security key {}", key.name))
            }
            Self::UserAdded { user } => {
                let self_enrollment_enabled = !user.is_enrolled();
                let enrollment_flag_text = if self_enrollment_enabled {
                    "enabled"
                } else {
                    "disabled"
                };
                Some(format!(
                    "Added user {user} with email {} and self-enrollment {enrollment_flag_text}",
                    user.email
                ))
            }
            Self::UserRemoved { user } => Some(format!("Removed user {user}")),
            Self::UserModified { before, after } => {
                let mut description = format!("Modified user {after}");

                // check if status has changed
                if before.is_active != after.is_active {
                    let status_change_text = if after.is_active {
                        "enabled"
                    } else {
                        "disabled"
                    };
                    description = format!("{description}, status changed to {status_change_text}");
                }
                Some(description)
            }
            Self::UserGroupsModified {
                user,
                before,
                after,
            } => Some(format!(
                "User groups modified! User:{user} Before: {before:?} After {after:?}"
            )),
            Self::UserDeviceAdded { owner, device } => {
                Some(format!("Added device {device} for user {owner}"))
            }
            Self::UserDeviceRemoved { owner, device } => {
                Some(format!("Removed device {device} owned by user {owner}"))
            }
            Self::UserDeviceModified {
                owner,
                before: _,
                after,
            } => Some(format!("Modified device {after} owned by user {owner}")),
            Self::NetworkDeviceAdded { device, location } => Some(format!(
                "Added network device {device} to location {location}"
            )),
            Self::NetworkDeviceRemoved { device, location } => Some(format!(
                "Removed network device {device} from location {location}"
            )),
            Self::NetworkDeviceModified {
                before: _,
                after,
                location,
            } => Some(format!(
                "Modified network device {after} in location {location}"
            )),
            Self::ActivityLogStreamCreated { stream } => Some(format!(
                "Created {} activity log stream {}",
                stream.stream_type, stream.name
            )),
            Self::ActivityLogStreamModified { before: _, after } => Some(format!(
                "Modified {} activity log stream {}",
                after.stream_type, after.name
            )),
            Self::ActivityLogStreamRemoved { stream } => Some(format!(
                "Removed {} activity log stream {}",
                stream.stream_type, stream.name
            )),
            Self::VpnLocationAdded { location } => Some(format!("Added VPN location {location}")),
            Self::VpnLocationRemoved { location } => {
                Some(format!("Removed VPN location {location}"))
            }
            Self::VpnLocationModified { before: _, after } => {
                Some(format!("VPN location {after} was modified"))
            }
            Self::ApiTokenAdded { owner, token } => {
                Some(format!("Added API token {} for user {owner}", token.name))
            }
            Self::ApiTokenRemoved { owner, token } => Some(format!(
                "Removed API token {} owned by user {owner}",
                token.name
            )),
            Self::ApiTokenRenamed {
                owner,
                token: _,
                old_name,
                new_name,
            } => Some(format!(
                "API token owned by user {owner} was renamed from {old_name} to {new_name}",
            )),
            Self::OpenIdAppAdded { app } => Some(format!("Added OpenID application {}", app.name)),
            Self::OpenIdAppRemoved { app } => {
                Some(format!("Removed OpenID application {}", app.name))
            }
            Self::OpenIdAppModified { before: _, after } => {
                Some(format!("Modified OpenID application {}", after.name))
            }
            Self::OpenIdAppStateChanged { app, enabled } => {
                let state = if *enabled { "Enabled" } else { "Disabled" };
                Some(format!("{} OpenID application {}", state, app.name))
            }
            Self::OpenIdProviderModified { provider } => {
                Some(format!("Modified OpenID provider {}", provider.name))
            }
            Self::OpenIdProviderRemoved { provider } => {
                Some(format!("Removed OpenID provider {}", provider.name))
            }
            Self::SettingsUpdated {
                before: _,
                after: _,
            } => None,
            Self::SettingsUpdatedPartial {
                before: _,
                after: _,
            } => None,
            Self::SettingsDefaultBrandingRestored => {
                Some("Restored default branding settings".to_owned())
            }
            Self::GroupsBulkAssigned { users, groups } => Some(format!(
                "Assigned {} users to {} groups",
                users.len(),
                groups.len()
            )),
            Self::GroupAdded { group } => Some(format!("Added group {}", group.name)),
            Self::GroupModified { before: _, after } => {
                Some(format!("Modified group {}", after.name))
            }
            Self::GroupRemoved { group } => Some(format!("Removed group {}", group.name)),
            Self::GroupMemberAdded { group, user } => {
                Some(format!("Added user {user} to group {}", group.name))
            }
            Self::GroupMemberRemoved { group, user } => {
                Some(format!("Removed user {user} from group {}", group.name))
            }
            Self::GroupMembersModified {
                group,
                added,
                removed,
            } => Some(format!(
                "Added: {}, Removed: {}, for group {}",
                added
                    .iter()
                    .map(|user| user.username.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                removed
                    .iter()
                    .map(|user| user.username.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                group.name
            )),
            Self::WebHookAdded { webhook } => {
                Some(format!("Added webhook with URL {}", webhook.url))
            }
            Self::WebHookModified { before: _, after } => {
                Some(format!("Modified webhook with URL {}", after.url))
            }
            Self::WebHookRemoved { webhook } => {
                Some(format!("Removed webhook with ULR {}", webhook.url))
            }
            Self::WebHookStateChanged { webhook, enabled } => {
                let state = if *enabled { "Enabled" } else { "Disabled" };
                Some(format!("{} webhook with URL {}", state, webhook.url))
            }
            Self::AuthenticationKeyAdded { key } => Some(format!(
                "Added {} authentication key {}",
                key.key_type,
                key.name.clone().unwrap_or_default()
            )),
            Self::AuthenticationKeyRemoved { key } => Some(format!(
                "Removed {} authentication key {}",
                key.key_type,
                key.name.clone().unwrap_or_default()
            )),
            Self::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            } => Some(format!(
                "Renamed {} authentication key from {} to {}",
                key.key_type,
                old_name.clone().unwrap_or_default(),
                new_name.clone().unwrap_or_default()
            )),
            Self::ClientConfigurationTokenAdded { user } => {
                Some(format!("Added client configuration token for user {user}"))
            }
            Self::UserSnatBindingAdded { user, binding } => Some(format!(
                "Devices owned by user {user} bound to public IP {}",
                binding.public_ip
            )),
            Self::UserSnatBindingRemoved { user, binding } => Some(format!(
                "Removed public IP {} binding for user {user}",
                binding.public_ip
            )),
            Self::UserSnatBindingModified {
                user,
                before,
                after,
            } => Some(format!(
                "Public IP bound to devices owned by user {user} changed from {} to {}",
                before.public_ip, after.public_ip
            )),
            Self::ProxyModified { before: _, after } => Some(format!("Modified proxy {after}")),
            Self::ProxyDeleted { proxy } => Some(format!("Deleted proxy {proxy}")),
            Self::GatewayModified { before: _, after } => Some(format!("Modified gateway {after}")),
            Self::GatewayDeleted { gateway } => Some(format!("Deleted gateway {gateway}")),
            Self::DevicePostureCreated { snapshot } => Some(format!(
                "Created device posture check {}",
                snapshot.device_posture.name
            )),
            Self::DevicePostureUpdated { after, .. } => Some(format!(
                "Updated device posture check {}",
                after.device_posture.name
            )),
            Self::DevicePostureDeleted { snapshot } => Some(format!(
                "Deleted device posture check {}",
                snapshot.device_posture.name
            )),
            Self::DevicePostureDuplicated { duplicate, .. } => Some(format!(
                "Duplicated device posture check as {}",
                duplicate.device_posture.name
            )),
            Self::DevicePostureLocationsAssigned {
                posture_id,
                location_ids,
            } => Some(format!(
                "Assigned {} location(s) to device posture check {posture_id}",
                location_ids.len()
            )),
            Self::LocationPosturesAssigned {
                location_id,
                posture_ids,
            } => Some(format!(
                "Assigned {} posture check(s) to location {location_id}",
                posture_ids.len()
            )),
        }
    }
}

impl ClientEvent {
    #[must_use]
    pub fn description(&self) -> Option<String> {
        match self {
            // FIXME: currently unused
            Self::DesktopClientActivated { .. } | Self::DesktopClientUpdated { .. } => None,
            Self::DevicePostureCheckPassed {
                device_id,
                device_name,
            } => Some(format!(
                "Device posture check passed for device {device_name} ({device_id})"
            )),
            Self::DevicePostureCheckFailed {
                device_id,
                device_name,
                failed_checks,
            } => Some(format!(
                "Device posture check failed for device {device_name} ({device_id}): {}",
                failed_checks.join(",")
            )),
        }
    }
}

impl VpnEvent {
    #[must_use]
    pub fn description(&self) -> Option<String> {
        match self {
            Self::ClientMfaSuccess {
                location,
                device,
                method,
            } => Some(format!(
                "Device {device} completed MFA authorization for location {location} using {method}"
            )),
            Self::ClientMfaFailed {
                location,
                device,
                method,
                message,
            } => Some(format!(
                "Device {device} failed to connect to MFA location {location} using {method} with: {message}"
            )),
            Self::ConnectedToLocation { location, device } => {
                Some(format!("Device {device} connected to location {location}"))
            }
            Self::DisconnectedFromLocation { location, device } => Some(format!(
                "Device {device} disconnected from location {location}"
            )),
            Self::MfaConnectedToLocation { location, device } => Some(format!(
                "Device {device} connected to MFA location {location}"
            )),
            Self::MfaDisconnectedFromLocation { location, device } => Some(format!(
                "Device {device} disconnected from MFA location {location}"
            )),
        }
    }
}

impl EnrollmentEvent {
    #[must_use]
    pub fn description(&self) -> Option<String> {
        match self {
            Self::EnrollmentStarted => Some("User started enrollment process".to_owned()),
            Self::EnrollmentDeviceAdded { device } => {
                Some(format!("Added device {} during enrollment", device.name))
            }
            Self::EnrollmentCompleted => Some("User completed enrollment process".to_owned()),
            Self::PasswordResetRequested
            | Self::PasswordResetStarted
            | Self::PasswordResetCompleted => None,
            Self::TokenAdded { user } => Some(format!("Added enrollment token for user {user}")),
        }
    }
}
