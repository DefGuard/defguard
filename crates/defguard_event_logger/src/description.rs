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
            };
            Some(description)
        }
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
        DefguardEvent::ActivityLogStreamCreated { stream } => todo!(),
        DefguardEvent::ActivityLogStreamModified { before, after } => todo!(),
        DefguardEvent::ActivityLogStreamRemoved { stream } => todo!(),
        DefguardEvent::VpnLocationAdded { location } => todo!(),
        DefguardEvent::VpnLocationRemoved { location } => todo!(),
        DefguardEvent::VpnLocationModified { before, after } => todo!(),
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
        DefguardEvent::OpenIdAppAdded { app } => todo!(),
        DefguardEvent::OpenIdAppRemoved { app } => todo!(),
        DefguardEvent::OpenIdAppModified { before, after } => todo!(),
        DefguardEvent::OpenIdAppStateChanged { app, enabled } => todo!(),
        DefguardEvent::OpenIdProviderModified { provider } => todo!(),
        DefguardEvent::OpenIdProviderRemoved { provider } => todo!(),
        DefguardEvent::SettingsUpdated => todo!(),
        DefguardEvent::SettingsUpdatedPartial => todo!(),
        DefguardEvent::SettingsDefaultBrandingRestored => todo!(),
        DefguardEvent::GroupsBulkAssigned { users, groups } => todo!(),
        DefguardEvent::GroupAdded { group } => todo!(),
        DefguardEvent::GroupModified { before, after } => todo!(),
        DefguardEvent::GroupRemoved { group } => todo!(),
        DefguardEvent::GroupMemberAdded { group, user } => todo!(),
        DefguardEvent::GroupMemberRemoved { group, user } => todo!(),
        DefguardEvent::WebHookAdded { webhook } => todo!(),
        DefguardEvent::WebHookModified { before, after } => todo!(),
        DefguardEvent::WebHookRemoved { webhook } => todo!(),
        DefguardEvent::WebHookStateChanged { webhook, enabled } => todo!(),
        DefguardEvent::AuthenticationKeyAdded { key } => todo!(),
        DefguardEvent::AuthenticationKeyRemoved { key } => todo!(),
        DefguardEvent::AuthenticationKeyRenamed {
            key,
            old_name,
            new_name,
        } => todo!(),
        DefguardEvent::EnrollmentTokenAdded { user } => todo!(),
        DefguardEvent::ClientConfigurationTokenAdded { user } => todo!(),
        DefguardEvent::UserSnatBindingAdded { user, binding } => Some(format!(
            "Devices owned by user {} bound to public IP {}",
            user.username, binding.public_ip
        )),
        DefguardEvent::UserSnatBindingRemoved { user, binding } => todo!(),
        DefguardEvent::UserSnatBindingModified {
            user,
            before,
            after,
        } => Some(format!(
            "Public IP bound to devices owned by user {} changed from {} to {}",
            user.username, before.public_ip, after.public_ip
        )),
    }
}

pub fn get_vpn_event_description(event: &VpnEvent) -> Option<String> {
    todo!()
}

pub fn get_enrollment_event_description(event: &EnrollmentEvent) -> Option<String> {
    todo!()
}
