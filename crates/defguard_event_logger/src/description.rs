//! Event description generation for activity log.
//!
//! This module provides functions to generate human-readable descriptions for various
//! types of events that occur within the system. These descriptions are used to provide usable
//! context about what happened during each event.
//!
//! Each event type has its own description generator function that takes the event data
//! and returns an optional description string. Some events may not require additional
//! description beyond their event type name, in which case `None` is returned.

use crate::message::{DefguardEvent, EnrollmentEvent, VpnEvent};

#[must_use]
pub fn get_defguard_event_description(event: &DefguardEvent) -> Option<String> {
    match event {
        DefguardEvent::UserLogin => None,
        DefguardEvent::UserLoginFailed { message } => {
            Some(format!("User login failed with: {message}"))
        }
        DefguardEvent::UserMfaLogin { mfa_method } => {
            Some(format!("User logged in using {mfa_method}"))
        }
        DefguardEvent::UserMfaLoginFailed {
            mfa_method,
            message,
        } => Some(format!(
            "User login using {mfa_method} failed with: {message}"
        )),
        DefguardEvent::UserLogout => None,
        DefguardEvent::RecoveryCodeUsed => None,
        DefguardEvent::PasswordChanged => None,
        DefguardEvent::MfaDisabled => Some("Disabled own MFA".to_string()),
        DefguardEvent::UserMfaDisabled { user } => Some(format!("Disabled MFA for user {user}")),
        DefguardEvent::MfaTotpEnabled => Some("User configured TOTP for MFA".to_string()),
        DefguardEvent::MfaTotpDisabled => Some("User disabled TOTP for MFA".to_string()),
        DefguardEvent::MfaEmailEnabled => Some("User configured email for MFA".to_string()),
        DefguardEvent::MfaEmailDisabled => Some("User disabled email for MFA".to_string()),
        DefguardEvent::PasswordChangedByAdmin { user } => {
            Some(format!("Password for user {user} was changed by an admin"))
        }
        DefguardEvent::PasswordReset { user } => {
            Some(format!("Password for user {user} was reset"))
        }
        DefguardEvent::MfaSecurityKeyAdded { key } => {
            Some(format!("Added MFA security key {}", key.name))
        }
        DefguardEvent::MfaSecurityKeyRemoved { key } => {
            Some(format!("Removed MFA security key {}", key.name))
        }
        DefguardEvent::UserAdded { user } => {
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
        DefguardEvent::UserRemoved { user } => Some(format!("Removed user {user}")),
        DefguardEvent::UserModified { before, after } => {
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
        DefguardEvent::UserGroupsModified {
            user,
            before,
            after,
        } => Some(format!(
            "User groups modified! User:{user} Before: {before:?} After {after:?}"
        )),
        DefguardEvent::UserDeviceAdded { owner, device } => {
            Some(format!("Added device {device} for user {owner}"))
        }
        DefguardEvent::UserDeviceRemoved { owner, device } => {
            Some(format!("Removed device {device} owned by user {owner}"))
        }
        DefguardEvent::UserDeviceModified {
            owner,
            before: _,
            after,
        } => Some(format!("Modified device {after} owned by user {owner}")),
        DefguardEvent::NetworkDeviceAdded { device, location } => Some(format!(
            "Added network device {device} to location {location}"
        )),
        DefguardEvent::NetworkDeviceRemoved { device, location } => Some(format!(
            "Removed network device {device} from location {location}"
        )),
        DefguardEvent::NetworkDeviceModified {
            before: _,
            after,
            location,
        } => Some(format!(
            "Modified network device {after} in location {location}"
        )),
        DefguardEvent::ActivityLogStreamCreated { stream } => Some(format!(
            "Created {} activity log stream {}",
            stream.stream_type, stream.name
        )),
        DefguardEvent::ActivityLogStreamModified { before: _, after } => Some(format!(
            "Modified {} activity log stream {}",
            after.stream_type, after.name
        )),
        DefguardEvent::ActivityLogStreamRemoved { stream } => Some(format!(
            "Removed {} activity log stream {}",
            stream.stream_type, stream.name
        )),
        DefguardEvent::VpnLocationAdded { location } => {
            Some(format!("Added VPN location {location}"))
        }
        DefguardEvent::VpnLocationRemoved { location } => {
            Some(format!("Removed VPN location {location}"))
        }
        DefguardEvent::VpnLocationModified { before: _, after } => {
            Some(format!("VPN location {after} was modified"))
        }
        DefguardEvent::ApiTokenAdded { owner, token } => {
            Some(format!("Added API token {} for user {owner}", token.name))
        }
        DefguardEvent::ApiTokenRemoved { owner, token } => Some(format!(
            "Removed API token {} owned by user {owner}",
            token.name
        )),
        DefguardEvent::ApiTokenRenamed {
            owner,
            token: _,
            old_name,
            new_name,
        } => Some(format!(
            "API token owned by user {owner} was renamed from {old_name} to {new_name}",
        )),
        DefguardEvent::OpenIdAppAdded { app } => {
            Some(format!("Added OpenID application {}", app.name))
        }
        DefguardEvent::OpenIdAppRemoved { app } => {
            Some(format!("Removed OpenID application {}", app.name))
        }
        DefguardEvent::OpenIdAppModified { before: _, after } => {
            Some(format!("Modified OpenID application {}", after.name))
        }
        DefguardEvent::OpenIdAppStateChanged { app, enabled } => {
            let state = if *enabled { "Enabled" } else { "Disabled" };
            Some(format!("{} OpenID application {}", state, app.name))
        }
        DefguardEvent::OpenIdProviderModified { provider } => {
            Some(format!("Modified OpenID provider {}", provider.name))
        }
        DefguardEvent::OpenIdProviderRemoved { provider } => {
            Some(format!("Removed OpenID provider {}", provider.name))
        }
        DefguardEvent::SettingsUpdated {
            before: _,
            after: _,
        } => None,
        DefguardEvent::SettingsUpdatedPartial {
            before: _,
            after: _,
        } => None,
        DefguardEvent::SettingsDefaultBrandingRestored => {
            Some("Restored default branding settings".to_string())
        }
        DefguardEvent::GroupsBulkAssigned { users, groups } => Some(format!(
            "Assigned {} users to {} groups",
            users.len(),
            groups.len()
        )),
        DefguardEvent::GroupAdded { group } => Some(format!("Added group {}", group.name)),
        DefguardEvent::GroupModified { before: _, after } => {
            Some(format!("Modified group {}", after.name))
        }
        DefguardEvent::GroupRemoved { group } => Some(format!("Removed group {}", group.name)),
        DefguardEvent::GroupMemberAdded { group, user } => {
            Some(format!("Added user {user} to group {}", group.name))
        }
        DefguardEvent::GroupMemberRemoved { group, user } => {
            Some(format!("Removed user {user} from group {}", group.name))
        }
        DefguardEvent::GroupMembersModified {
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
        DefguardEvent::WebHookAdded { webhook } => {
            Some(format!("Added webhook with URL {}", webhook.url))
        }
        DefguardEvent::WebHookModified { before: _, after } => {
            Some(format!("Modified webhook with URL {}", after.url))
        }
        DefguardEvent::WebHookRemoved { webhook } => {
            Some(format!("Removed webhook with ULR {}", webhook.url))
        }
        DefguardEvent::WebHookStateChanged { webhook, enabled } => {
            let state = if *enabled { "Enabled" } else { "Disabled" };
            Some(format!("{} webhook with URL {}", state, webhook.url))
        }
        DefguardEvent::AuthenticationKeyAdded { key } => Some(format!(
            "Added {} authentication key {}",
            key.key_type,
            key.name.clone().unwrap_or_default()
        )),
        DefguardEvent::AuthenticationKeyRemoved { key } => Some(format!(
            "Removed {} authentication key {}",
            key.key_type,
            key.name.clone().unwrap_or_default()
        )),
        DefguardEvent::AuthenticationKeyRenamed {
            key,
            old_name,
            new_name,
        } => Some(format!(
            "Renamed {} authentication key from {} to {}",
            key.key_type,
            old_name.clone().unwrap_or_default(),
            new_name.clone().unwrap_or_default()
        )),
        DefguardEvent::ClientConfigurationTokenAdded { user } => {
            Some(format!("Added client configuration token for user {user}",))
        }
        DefguardEvent::UserSnatBindingAdded { user, binding } => Some(format!(
            "Devices owned by user {user} bound to public IP {}",
            binding.public_ip
        )),
        DefguardEvent::UserSnatBindingRemoved { user, binding } => Some(format!(
            "Removed public IP {} binding for user {user}",
            binding.public_ip
        )),
        DefguardEvent::UserSnatBindingModified {
            user,
            before,
            after,
        } => Some(format!(
            "Public IP bound to devices owned by user {user} changed from {} to {}",
            before.public_ip, after.public_ip
        )),
        DefguardEvent::ProxyModified { before: _, after } => {
            Some(format!("Modified proxy {after}"))
        }
        DefguardEvent::ProxyDeleted { proxy } => Some(format!("Deleted proxy {proxy}")),
        DefguardEvent::GatewayModified { before: _, after } => {
            Some(format!("Modified gateway {after}"))
        }
        DefguardEvent::GatewayDeleted { gateway } => Some(format!("Deleted gateway {gateway}")),
    }
}

#[must_use]
pub fn get_vpn_event_description(event: &VpnEvent) -> Option<String> {
    match event {
        VpnEvent::ClientMfaSuccess {
            location,
            device,
            method,
        } => Some(format!(
            "Device {device} completed MFA authorization for location {location} using {method}"
        )),
        VpnEvent::ClientMfaFailed {
            location,
            device,
            method,
            message,
        } => Some(format!(
            "Device {device} failed to connect to MFA location {location} using {method} with: {message}"
        )),
        VpnEvent::ConnectedToLocation { location, device } => {
            Some(format!("Device {device} connected to location {location}"))
        }
        VpnEvent::DisconnectedFromLocation { location, device } => Some(format!(
            "Device {device} disconnected from location {location}"
        )),
    }
}

#[must_use]
pub fn get_enrollment_event_description(event: &EnrollmentEvent) -> Option<String> {
    match event {
        EnrollmentEvent::EnrollmentStarted => Some("User started enrollment process".to_string()),
        EnrollmentEvent::EnrollmentDeviceAdded { device } => {
            Some(format!("Added device {} during enrollment", device.name))
        }
        EnrollmentEvent::EnrollmentCompleted => {
            Some("User completed enrollment process".to_string())
        }
        EnrollmentEvent::PasswordResetRequested => None,
        EnrollmentEvent::PasswordResetStarted => None,
        EnrollmentEvent::PasswordResetCompleted => None,
        EnrollmentEvent::TokenAdded { user } => {
            Some(format!("Added enrollment token for user {user}"))
        }
    }
}
