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
        DefguardEvent::MfaTotpEnabled => todo!(),
        DefguardEvent::MfaTotpDisabled => todo!(),
        DefguardEvent::MfaEmailEnabled => todo!(),
        DefguardEvent::MfaEmailDisabled => todo!(),
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
        DefguardEvent::UserAdded { user } => todo!(),
        DefguardEvent::UserRemoved { user } => todo!(),
        DefguardEvent::UserModified { before, after } => todo!(),
        DefguardEvent::UserDeviceAdded { owner, device } => todo!(),
        DefguardEvent::UserDeviceRemoved { owner, device } => todo!(),
        DefguardEvent::UserDeviceModified {
            owner,
            before,
            after,
        } => todo!(),
        DefguardEvent::NetworkDeviceAdded { device, location } => todo!(),
        DefguardEvent::NetworkDeviceRemoved { device, location } => todo!(),
        DefguardEvent::NetworkDeviceModified {
            before,
            after,
            location,
        } => todo!(),
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
